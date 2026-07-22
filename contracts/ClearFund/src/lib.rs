#![no_std]

//! ClearFund — Milestone-Based Charity Transparency Escrow
//!
//! Donors deposit USDC (or any SEP-41 token) into a campaign escrow.
//! Funds are locked per-milestone and only released to the NGO wallet
//! when a whitelisted auditor approves that specific milestone on-chain.
//! This gives donors a fully auditable, tamper-proof trail of exactly
//! how their money was disbursed.

use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype, token, Address, Env, Symbol, Vec,
};

/// Storage keys used across the contract.
#[contracttype]
pub enum DataKey {
    /// Monotonically increasing counter used to assign new campaign ids.
    CampaignCount,
    /// A single campaign, keyed by its id.
    Campaign(u32),
}

/// A single funding milestone within a campaign.
#[derive(Clone)]
#[contracttype]
pub struct Milestone {
    /// Short human-readable description, e.g. "500_food_kits".
    pub description: Symbol,
    /// Amount (in token base units) released when this milestone is approved.
    pub amount: i128,
    /// Whether this milestone has already been paid out.
    pub released: bool,
}

/// A charity campaign: who runs it, who audits it, and its milestone schedule.
#[derive(Clone)]
#[contracttype]
pub struct Campaign {
    pub id: u32,
    /// The NGO wallet that receives released funds.
    pub ngo: Address,
    /// The independent auditor allowed to approve milestones.
    pub auditor: Address,
    /// The SEP-41 token contract address used for donations (e.g. USDC).
    pub token: Address,
    /// Sum of all milestone amounts — the campaign's funding target.
    pub target: i128,
    /// Total amount donated so far (locked in escrow, before release).
    pub raised: i128,
    /// Ordered list of milestones for this campaign.
    pub milestones: Vec<Milestone>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    CampaignNotFound = 1,
    InvalidMilestoneIndex = 2,
    MilestoneAlreadyReleased = 3,
    InsufficientEscrowBalance = 4,
    NoMilestonesProvided = 5,
}

#[contract]
pub struct ClearFundContract;

#[contractimpl]
impl ClearFundContract {
    /// Creates a new campaign with a fixed milestone schedule.
    /// `milestone_amounts` and `milestone_descriptions` must be the same length
    /// and in the same order. Returns the new campaign's id.
    ///
    /// Why: the milestone schedule is fixed at creation time so donors know
    /// exactly what they are funding before they send any money.
    pub fn create_campaign(
        env: Env,
        ngo: Address,
        auditor: Address,
        token: Address,
        milestone_descriptions: Vec<Symbol>,
        milestone_amounts: Vec<i128>,
    ) -> Result<u32, Error> {
        ngo.require_auth();

        if milestone_amounts.is_empty() || milestone_descriptions.is_empty() {
            return Err(Error::NoMilestonesProvided);
        }

        let mut milestones: Vec<Milestone> = Vec::new(&env);
        let mut target: i128 = 0;
        for i in 0..milestone_amounts.len() {
            let amount = milestone_amounts.get(i).unwrap();
            let description = milestone_descriptions.get(i).unwrap();
            target += amount;
            milestones.push_back(Milestone {
                description,
                amount,
                released: false,
            });
        }

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CampaignCount)
            .unwrap_or(0);
        let id = count + 1;

        let campaign = Campaign {
            id,
            ngo,
            auditor,
            token,
            target,
            raised: 0,
            milestones,
        };

        env.storage().instance().set(&DataKey::Campaign(id), &campaign);
        env.storage().instance().set(&DataKey::CampaignCount, &id);

        Ok(id)
    }

    /// Donor sends `amount` of the campaign's token into escrow (this contract's balance).
    /// Funds sit locked here until an auditor approves a milestone.
    ///
    /// Why: pulling funds into the contract's own balance (rather than the NGO's
    /// wallet directly) is what makes the escrow trustless — the NGO physically
    /// cannot access the money until a milestone is verified.
    pub fn donate(env: Env, donor: Address, campaign_id: u32, amount: i128) -> Result<(), Error> {
        donor.require_auth();

        let mut campaign: Campaign = env
            .storage()
            .instance()
            .get(&DataKey::Campaign(campaign_id))
            .ok_or(Error::CampaignNotFound)?;

        let token_client = token::Client::new(&env, &campaign.token);
        token_client.transfer(&donor, &env.current_contract_address(), &amount);

        campaign.raised += amount;
        env.storage()
            .instance()
            .set(&DataKey::Campaign(campaign_id), &campaign);

        Ok(())
    }

    /// Auditor confirms that a milestone has been completed in the real world.
    /// On success, the milestone's tranche is transferred from escrow to the NGO
    /// wallet immediately, and the milestone is marked released so it can never
    /// be paid twice.
    ///
    /// Why: only the auditor (not the NGO, not the donor) can trigger release —
    /// this is the core transparency guarantee of the whole system.
    pub fn approve_milestone(
        env: Env,
        campaign_id: u32,
        milestone_index: u32,
    ) -> Result<(), Error> {
        let mut campaign: Campaign = env
            .storage()
            .instance()
            .get(&DataKey::Campaign(campaign_id))
            .ok_or(Error::CampaignNotFound)?;

        campaign.auditor.require_auth();

        let idx = milestone_index as u32;
        if idx >= campaign.milestones.len() {
            return Err(Error::InvalidMilestoneIndex);
        }

        let mut milestone = campaign.milestones.get(idx).unwrap();
        if milestone.released {
            return Err(Error::MilestoneAlreadyReleased);
        }
        if campaign.raised < milestone.amount {
            return Err(Error::InsufficientEscrowBalance);
        }

        let token_client = token::Client::new(&env, &campaign.token);
        token_client.transfer(
            &env.current_contract_address(),
            &campaign.ngo,
            &milestone.amount,
        );

        milestone.released = true;
        campaign.milestones.set(idx, milestone.clone());
        campaign.raised -= milestone.amount;

        env.storage()
            .instance()
            .set(&DataKey::Campaign(campaign_id), &campaign);

        env.events().publish(
            (Symbol::new(&env, "milestone_released"), campaign_id),
            (milestone_index, milestone.amount),
        );

        Ok(())
    }

    /// Read-only lookup so donors (or a frontend) can verify campaign state at any time.
    pub fn get_campaign(env: Env, campaign_id: u32) -> Result<Campaign, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Campaign(campaign_id))
            .ok_or(Error::CampaignNotFound)
    }
}

mod test;