#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, Env, Symbol,
};

/// Storage key types for the RemitLink escrow contract.
#[contracttype]
pub enum DataKey {
    /// Maps transfer_id → Transfer struct
    Transfer(u64),
    /// Monotonic counter for transfer IDs
    Counter,
}

/// Represents a single remittance escrow transfer.
#[contracttype]
#[derive(Clone)]
pub struct Transfer {
    /// The OFW (sender) who locked USDC into escrow
    pub sender: Address,
    /// The recipient (family member) who will claim the USDC
    pub recipient: Address,
    /// USDC amount in stroops (1 USDC = 10_000_000)
    pub amount: i128,
    /// Ledger number at which the rate-lock expires (sender can refund after this)
    pub expiry_ledger: u32,
    /// Whether the transfer has been claimed or refunded
    pub claimed: bool,
}

#[contract]
pub struct RemitLinkContract;

#[contractimpl]
impl RemitLinkContract {
    /// Called by the OFW to lock USDC in escrow for a recipient.
    /// The rate-lock window is 720 ledgers (~1 hour on Stellar).
    ///
    /// # Arguments
    /// * `sender`    - OFW's Stellar address (must authorize this call)
    /// * `recipient` - Family member's Stellar address
    /// * `token`     - USDC contract address on this network
    /// * `amount`    - Amount in USDC stroops to send
    pub fn create_transfer(
        env: Env,
        sender: Address,
        recipient: Address,
        token: Address,
        amount: i128,
    ) -> u64 {
        // Sender must sign this transaction
        sender.require_auth();

        // Validate amount is positive
        assert!(amount > 0, "Amount must be positive");

        // Pull USDC from sender into this contract
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        // Generate a unique transfer ID
        let id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::Counter)
            .unwrap_or(0u64)
            + 1;
        env.storage().instance().set(&DataKey::Counter, &id);

        // Store the transfer with a 720-ledger expiry window
        let expiry_ledger = env.ledger().sequence() + 720;
        let transfer = Transfer {
            sender: sender.clone(),
            recipient: recipient.clone(),
            amount,
            expiry_ledger,
            claimed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Transfer(id), &transfer);

        // Emit an event so frontends can listen
        env.events().publish(
            (Symbol::new(&env, "transfer_created"), sender),
            (id, recipient, amount),
        );

        id
    }

    /// Called by the recipient to claim their USDC before the rate-lock expires.
    ///
    /// # Arguments
    /// * `transfer_id` - ID returned by create_transfer
    /// * `token`       - USDC contract address
    pub fn claim(env: Env, transfer_id: u64, token: Address) {
        let mut transfer: Transfer = env
            .storage()
            .persistent()
            .get(&DataKey::Transfer(transfer_id))
            .expect("Transfer not found");

        // Only the designated recipient can claim
        transfer.recipient.require_auth();

        assert!(!transfer.claimed, "Transfer already claimed");
        // Must be claimed within the rate-lock window
        assert!(
            env.ledger().sequence() <= transfer.expiry_ledger,
            "Transfer expired — sender can now refund"
        );

        // Send USDC from contract to recipient
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(
            &env.current_contract_address(),
            &transfer.recipient,
            &transfer.amount,
        );

        // Mark as claimed to prevent double-spend
        transfer.claimed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Transfer(transfer_id), &transfer);

        env.events().publish(
            (Symbol::new(&env, "transfer_claimed"), transfer.recipient),
            (transfer_id, transfer.amount),
        );
    }

    /// Called by the sender to recover USDC after the rate-lock window expires
    /// (e.g., if the recipient's wallet wasn't set up yet).
    ///
    /// # Arguments
    /// * `transfer_id` - ID of the expired transfer
    /// * `token`       - USDC contract address
    pub fn refund(env: Env, transfer_id: u64, token: Address) {
        let mut transfer: Transfer = env
            .storage()
            .persistent()
            .get(&DataKey::Transfer(transfer_id))
            .expect("Transfer not found");

        // Only the original sender can refund
        transfer.sender.require_auth();

        assert!(!transfer.claimed, "Transfer already claimed — cannot refund");
        // Refund only available after expiry
        assert!(
            env.ledger().sequence() > transfer.expiry_ledger,
            "Rate-lock window still active"
        );

        // Return USDC to sender
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(
            &env.current_contract_address(),
            &transfer.sender,
            &transfer.amount,
        );

        // Mark claimed to prevent re-use
        transfer.claimed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Transfer(transfer_id), &transfer);

        env.events().publish(
            (Symbol::new(&env, "transfer_refunded"), transfer.sender),
            (transfer_id, transfer.amount),
        );
    }

    /// Read-only: fetch transfer details by ID.
    pub fn get_transfer(env: Env, transfer_id: u64) -> Transfer {
        env.storage()
            .persistent()
            .get(&DataKey::Transfer(transfer_id))
            .expect("Transfer not found")
    }
}