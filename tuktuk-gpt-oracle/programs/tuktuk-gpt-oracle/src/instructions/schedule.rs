use anchor_lang::{prelude::*, InstructionData};
use anchor_lang::solana_program::instruction::Instruction;

use solana_gpt_oracle::ContextAccount;
use tuktuk_program::{
    TransactionSourceV0, 
    compile_transaction, 
    tuktuk::{
        cpi::{
            queue_task_v0, 
        }, 
        program::Tuktuk, 
        types::TriggerV0
    }
};

use crate::state::Agent;

#[derive(Accounts)]
pub struct Schedule<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Checked in oracle program
    #[account(mut)]
    pub interaction: UncheckedAccount<'info>,

    #[account(
        seeds = [b"agent", payer.key().as_ref()],
        bump = agent.bump,
    )]
    pub agent: Account<'info, Agent>,

    #[account(address = agent.context)]
    pub context_account: Account<'info, ContextAccount>,

    /// CHECK: Checked oracle id
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue: UncheckedAccount<'info>,

    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue_authority: UncheckedAccount<'info>,

    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: UncheckedAccount<'info>,

    /// CHECK: Via seeds
    #[account(
        mut,
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub tuktuk_program: Program<'info, Tuktuk>,
}

impl<'info> Schedule<'info> {
    pub fn schedule(&self, text: String, task_id: u16, bumps: &ScheduleBumps) -> Result<()> {
        let interact_ix = Instruction {
            program_id: crate::ID,
            accounts: vec![
                AccountMeta::new(self.payer.key(), false),
                AccountMeta::new(self.interaction.key(), false),
                AccountMeta::new_readonly(self.agent.key(), false),
                AccountMeta::new_readonly(self.context_account.key(), false),
                AccountMeta::new_readonly(self.oracle_program.key(), false),
                AccountMeta::new_readonly(self.system_program.key(), false),
            ],
            data: crate::instruction::InteractAgent { text }.data(),
        };

        let (compiled_tx, _) = compile_transaction(
            vec![interact_ix], 
            vec![]
        ).unwrap();

        queue_task_v0(
            CpiContext::new_with_signer(
                self.tuktuk_program.to_account_info(),
                tuktuk_program::tuktuk::cpi::accounts::QueueTaskV0 {
                    payer: self.payer.to_account_info(),
                    queue_authority: self.queue_authority.to_account_info(),
                    task_queue: self.task_queue.to_account_info(),
                    task_queue_authority: self.task_queue_authority.to_account_info(),
                    task: self.task.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                },
                &[&["queue_authority".as_bytes(), &[bumps.queue_authority]]],
            ),
            tuktuk_program::types::QueueTaskArgsV0 {
                id: task_id,
                trigger: TriggerV0::Now,
                transaction: TransactionSourceV0::CompiledV0(compiled_tx),
                crank_reward: Some(1_000_002),
                free_tasks: 0,
                description: format!("gpt-task-{}", task_id),
            },
        )?;

        Ok(())
    }
}