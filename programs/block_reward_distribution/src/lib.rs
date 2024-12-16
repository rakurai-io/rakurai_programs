use anchor_lang::prelude::*;

declare_id!("F6NzRRzmqx7j7xrQjVpc5AopCarPYK2JTZazaWWqXTeS");

#[program]
pub mod block_reward_distribution {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
