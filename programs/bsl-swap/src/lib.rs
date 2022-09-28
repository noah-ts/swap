pub mod state;

use anchor_lang::prelude::*;
use anchor_spl::{token::{Mint, Token, TokenAccount, Transfer, CloseAccount}};
use state::{UserState, SwapState, EscrowState, UserEnum};

declare_id!("FLjoHCAmjojgt7DUxid7WR9EyEj2Pysq9cMehRiM5vtp");

#[program]
pub mod bsl_swap {
    use super::*;

    pub fn initialize_user_state(ctx: Context<InitializeUserState>, bump: u8) -> Result<()> {
        let state = &mut ctx.accounts.user_state;
        state.bump = bump;
        Ok(())
    }

    pub fn initialize_swap_state(ctx: Context<InitializeSwapState>, swap_state_bump: u8) -> Result<()> {
        let state = &mut ctx.accounts.swap_state;
        state.swap_state_bump = swap_state_bump;
        Ok(())
    }

    pub fn initialize_escrow_state(ctx: Context<InitializeEscrowState>, bump: u8) -> Result<()> {
        let state = &mut ctx.accounts.escrow_state;
        state.state_bump = bump;
        Ok(())
    }

    pub fn initialize_escrow(ctx: Context<InitializeEscrow>, ata_bump: u8) -> Result<()> {
        let state = &mut ctx.accounts.escrow_state;
        state.escrow = ctx.accounts.escrow.key().clone();
        state.mint = ctx.accounts.mint.key().clone();
        state.ata_offeror = ctx.accounts.ata_offeror.key().clone();
        state.ata_bump = ata_bump;

        let swap_state = &mut ctx.accounts.swap_state;
        swap_state.mints_offeror.push(ctx.accounts.mint.key().clone());

        let transfer_instruction = Transfer{
            from: ctx.accounts.ata_offeror.to_account_info(),
            to: ctx.accounts.escrow.to_account_info(),
            authority: ctx.accounts.offeror.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        Ok(())
    }

    pub fn add_mint_offeree(ctx: Context<AddMintOfferee>) -> Result<()> {
        let state = &mut ctx.accounts.swap_state;
        state.mints_offeree.push(ctx.accounts.mint.key().clone());
        Ok(())
    }

    pub fn initiate_swap(ctx: Context<InitiateSwap>) -> Result<()> {
        let state = &mut ctx.accounts.swap_state;
        state.offeror = ctx.accounts.offeror.key().clone();
        state.offeree = ctx.accounts.offeree.key().clone();

        let offeror_state = &mut ctx.accounts.offeror_state;
        let offeree_state = &mut ctx.accounts.offeree_state;
        offeror_state.user_enum = UserEnum::Offeror.to_code();
        offeror_state.counter_party = ctx.accounts.offeree.key().clone();
        offeree_state.user_enum = UserEnum::Offeree.to_code();
        offeree_state.counter_party = ctx.accounts.offeror.key().clone();

        Ok(())
    }

    pub fn close_escrow(ctx: Context<CloseEscrow>) -> Result<()> {
        let bump_vector = &[ctx.accounts.swap_state.swap_state_bump][..];
        let inner = vec![
            b"swap_state".as_ref(),
            ctx.accounts.offeror.key.as_ref(),
            ctx.accounts.offeree.key.as_ref(),
            bump_vector.as_ref(),
        ];
        let outer = vec![inner.as_slice()];

        let transfer_instruction = Transfer{
            from: ctx.accounts.escrow.to_account_info(),
            to: ctx.accounts.ata.to_account_info(),
            authority: ctx.accounts.swap_state.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
            outer.as_slice(),
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        // Use the `reload()` function on an account to reload it's state. Since we performed the
        // transfer, we are expecting the `amount` field to have changed.
        let should_close = {
            ctx.accounts.escrow.reload()?;
            ctx.accounts.escrow.amount == 0
        };

        // If token account has no more tokens, it should be wiped out since it has no other use case.
        if should_close {
            let ca = CloseAccount{
                account: ctx.accounts.escrow.to_account_info(),
                destination: ctx.accounts.offeror.to_account_info(),
                authority: ctx.accounts.swap_state.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                ca,
                outer.as_slice(),
            );
            anchor_spl::token::close_account(cpi_ctx)?;
        }

        Ok(())
    }

    pub fn cancel_swap(ctx: Context<CancelSwap>) -> Result<()> {
        let offeror_state = &mut ctx.accounts.offeror_state;
        let offeree_state = &mut ctx.accounts.offeree_state;
        offeror_state.user_enum = UserEnum::None.to_code();
        offeree_state.user_enum = UserEnum::None.to_code();

        Ok(())
    }

    pub fn accept_swap(ctx: Context<AcceptSwap>) -> Result<()> {
        let offeror_state = &mut ctx.accounts.offeror_state;
        let offeree_state = &mut ctx.accounts.offeree_state;
        offeror_state.user_enum = UserEnum::None.to_code();
        offeree_state.user_enum = UserEnum::None.to_code();
        Ok(())
    }

    pub fn transfer_nft_from_offeree_to_offeror(ctx: Context<TransferNftFromOffereeToOfferor>) -> Result<()> {
        let transfer_instruction = Transfer{
            from: ctx.accounts.ata_offeree.to_account_info(),
            to: ctx.accounts.ata_offeror.to_account_info(),
            authority: ctx.accounts.offeree.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            transfer_instruction,
        );

        anchor_spl::token::transfer(cpi_ctx, 1)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeUserState<'info> {
    #[account(
        init,
        space = 50,
        payer = user,
        seeds=[b"user_state".as_ref(), user_seed.key().as_ref()],
        bump,
    )]
    user_state: Account<'info, UserState>,
    #[account(mut)]
    user: Signer<'info>,
    /// CHECK: not reading or writing to this account
    user_seed: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeSwapState<'info> {
    // PDAs
    #[account(
        init,
        space = 200,
        payer = offeror,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump,
    )]
    swap_state: Account<'info, SwapState>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeEscrowState<'info> {
    // PDAs
    #[account(
        mut,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        init,
        space = 32 * 3 + 1 + 8,
        payer = offeror,
        seeds=[b"escrow_state".as_ref(), offeror.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    escrow_state: Account<'info, EscrowState>,

    mint: Account<'info, Mint>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct InitializeEscrow<'info> {
    // PDAs
    #[account(
        mut,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"escrow_state".as_ref(), offeror.key().as_ref(), mint.key().as_ref()],
        bump = escrow_state.state_bump,
        has_one = escrow,
        has_one = mint,
    )]
    escrow_state: Account<'info, EscrowState>,
    #[account(
        init,
        payer = offeror,
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), mint.key().as_ref()],
        bump,
        token::mint=mint,
        token::authority=swap_state,
    )]
    escrow: Account<'info, TokenAccount>,

    mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint=ata_offeror.owner == offeror.key(),
        constraint=ata_offeror.mint == mint.key()
    )]
    ata_offeror: Account<'info, TokenAccount>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AddMintOfferee<'info> {
    // PDAs
    #[account(
        mut,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
    )]
    swap_state: Account<'info, SwapState>,

    mint: Account<'info, Mint>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitiateSwap<'info> {
    // PDAs
    #[account(
        mut,
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
    )]
    swap_state: Account<'info, SwapState>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,

    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeror.key().as_ref()],
        bump = offeror_state.bump,
    )]
    offeror_state: Account<'info, UserState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeree.key().as_ref()],
        bump = offeree_state.bump,
    )]
    offeree_state: Account<'info, UserState>,

    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CloseEscrow<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"escrow_state".as_ref(), offeror.key().as_ref(), mint.key().as_ref()],
        bump = escrow_state.state_bump,
        has_one = escrow,
        has_one = mint,
    )]
    escrow_state: Account<'info, EscrowState>,
    #[account(
        mut,
        seeds=[b"escrow".as_ref(), offeror.key().as_ref(), mint.key().as_ref()],
        bump = escrow_state.ata_bump,
    )]
    escrow: Account<'info, TokenAccount>,

    mint: Account<'info, Mint>,

    /// CHECK: only adding tokens to this account
    #[account(mut)]
    offeror: AccountInfo<'info>,
    /// CHECK: only adding tokens to this account
    #[account(mut)]
    offeree: AccountInfo<'info>,

    #[account(
        mut,
        constraint=ata.owner == offeror.key() || ata.owner == offeree.key(),
        constraint=ata.mint == mint.key()
    )]
    ata: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelSwap<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeror.key().as_ref()],
        bump = offeror_state.bump,
    )]
    offeror_state: Account<'info, UserState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeree.key().as_ref()],
        bump = offeree_state.bump,
    )]
    offeree_state: Account<'info, UserState>,

    #[account(mut)]
    offeror: Signer<'info>,
    /// CHECK: not reading or writing to this account 
    offeree: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct AcceptSwap<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
    )]
    swap_state: Account<'info, SwapState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeror.key().as_ref()],
        bump = offeror_state.bump,
    )]
    offeror_state: Account<'info, UserState>,
    #[account(
        mut,
        seeds=[b"user_state".as_ref(), offeree.key().as_ref()],
        bump = offeree_state.bump,
    )]
    offeree_state: Account<'info, UserState>,

    /// CHECK: not reading or writing to this account
    offeror: AccountInfo<'info>,
    /// CHECK: not reading or writing to this account
    offeree: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TransferNftFromOffereeToOfferor<'info> {
    // PDAs
    #[account(
        seeds=[b"swap_state".as_ref(), offeror.key().as_ref(), offeree.key().as_ref()],
        bump = swap_state.swap_state_bump,
        has_one = offeror,
        has_one = offeree,
    )]
    swap_state: Account<'info, SwapState>,

    mint: Account<'info, Mint>,

    /// CHECK: only adding tokens to this account
    #[account(mut)]
    offeror: AccountInfo<'info>,
    offeree: Signer<'info>,

    #[account(
        mut,
        constraint=ata_offeror.owner == offeror.key(),
        constraint=ata_offeror.mint == mint.key()
    )]
    ata_offeror: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint=ata_offeree.owner == offeree.key(),
        constraint=ata_offeree.mint == mint.key()
    )]
    ata_offeree: Account<'info, TokenAccount>,

    token_program: Program<'info, Token>,
}