use anchor_lang::{prelude::*, InstructionData, solana_program::instruction::Instruction};
use anchor_spl::{associated_token::AssociatedToken, token_interface::{Mint, TokenAccount, TokenInterface}};

use tuktuk_program::{
    TransactionSourceV0, 
    compile_transaction, 
    tuktuk::{
        cpi::{
            accounts::QueueTaskV0,
            queue_task_v0
        }, 
        program::Tuktuk, 
        types::TriggerV0
    }, types::QueueTaskArgsV0
};

use crate::{constants::AUTO_REFUND_TIME, state::Escrow};

#[derive(Accounts)]
pub struct Schedule<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        has_one = mint_a,
        has_one = maker,
        seeds = [b"escrow", maker.key().as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

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
    pub queue_authority: AccountInfo<'info>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}

impl<'info> Schedule<'info> {
    pub fn schedule(&mut self, task_id: u16, bumps: &ScheduleBumps) -> Result<()> {
        let (compiled_tx, _) = compile_transaction(
            vec![Instruction {
                program_id: crate::ID,
                accounts: crate::__cpi_client_accounts_refund_auto::RefundAuto {
                    maker: self.maker.to_account_info(), 
                    mint_a: self.mint_a.to_account_info(), 
                    maker_ata_a: self.maker_ata_a.to_account_info(), 
                    escrow: self.escrow.to_account_info(), 
                    vault: self.vault.to_account_info(), 
                    associated_token_program: self.associated_token_program.to_account_info(), 
                    token_program: self.token_program.to_account_info(), 
                    system_program: self.system_program.to_account_info(),
                }
                .to_account_metas(None)
                .to_vec(),
                data: crate::instruction::RefundAuto {}.data(),
            }],
        vec![],
        )?;

        queue_task_v0(
            CpiContext::new_with_signer(
                self.tuktuk_program.to_account_info(),
                QueueTaskV0 {
                    payer: self.maker.to_account_info(),
                    queue_authority: self.queue_authority.to_account_info(),
                    task_queue: self.task_queue.to_account_info(),
                    task_queue_authority: self.task_queue_authority.to_account_info(),
                    task: self.task.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                },
                &[&["queue_authority".as_bytes(), &[bumps.queue_authority]]],
            ),
            QueueTaskArgsV0 {
                trigger: TriggerV0::Timestamp(self.escrow.created_at + AUTO_REFUND_TIME),
                transaction: TransactionSourceV0::CompiledV0(compiled_tx),
                crank_reward: Some(1000001),
                free_tasks: 1,
                id: task_id,
                description: "refund_auto".to_string(),
            },
        )?;
        
    Ok(())
    }
}