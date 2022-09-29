import * as anchor from "@project-serum/anchor";
import { AnchorProvider, Program } from "@project-serum/anchor";
import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { clusterApiUrl, Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } from '@solana/web3.js';
import { BslSwap } from "../target/types/bsl_swap";
import idl from '../target/idl/bsl_swap.json'

const programId = idl.metadata.address

// this test tests on devnet, not local cluster
// private keys of offeror and offeree

const offerorPrivateKey = '2FuSkdDv2nQ9RBdVCjCrNTnMZo4prufJ1JUK7Go2D3eN6NU8R8gZ2A6GRkEnThTWpz9HJfcHEkiFWvqGfB5GfhWE'
const offereePrivateKey = '5T5jR82Z3z5JTq1hVqHSyogxq397tJVL2HYozYCo3CzLBHBTderH7Y1e3znmzmC1gCLtVNwqo1fcQaFyGheRqDcV'

class OfferorWallet {
  public publicKey: PublicKey
  constructor(public payer: Keypair) {
    this.publicKey = payer.publicKey
  }

  async signTransaction(tx: Transaction): Promise<Transaction> {
    tx.partialSign(this.payer);
    return tx;
  }

  async signAllTransactions(txs: Transaction[]): Promise<Transaction[]> {
    return txs.map((t) => {
      t.partialSign(this.payer);
      return t;
    });
  }
}

describe("bsl-swap", () => {
  const offeror = Keypair.fromSecretKey(bs58.decode(offerorPrivateKey))
  const offerorWallet = new OfferorWallet(offeror)
  const offeree = Keypair.fromSecretKey(bs58.decode(offereePrivateKey))
  const connection = new Connection(clusterApiUrl('devnet'))
  const provider = new AnchorProvider(connection, offerorWallet, { preflightCommitment: 'finalized' })
  const program = new Program(idl as any, programId, provider) as Program<BslSwap>

  const prepareForTest = async () => {
    const mintAssetA = new PublicKey('E9kyTxayTbR9GK42Q7Ub1i4UzBmQeN6EpTaKymfAAsi6')
    const mintAssetB = new PublicKey('GtTMuni1CE4stN5uQBRv2tcKQaQrs1Z2uDSM3fBxL2T6')

    const [offerorPdaState, offerorPdaBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('user_state'), offeror.publicKey.toBuffer()], program.programId
    )
    const [offereePdaState, offereePdaBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('user_state'), offeree.publicKey.toBuffer()], program.programId
    )
    const [swapState, swapBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('swap_state'), offeror.publicKey.toBuffer(), offeree.publicKey.toBuffer()], program.programId
    )
    const [escrowState, escrowStateBump] = await PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode('escrow_state'), offeror.publicKey.toBuffer(), mintAssetA.toBuffer()], program.programId
      )
    const [escrowAta, escrowAtaBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('escrow'), offeror.publicKey.toBuffer(), mintAssetA.toBuffer()], program.programId
    )
    console.log('Found 5 PDAs')

    const ataOfferorAssetA = await getAssociatedTokenAddress(mintAssetA, offeror.publicKey)
    const ataOfferorAssetB = await getAssociatedTokenAddress(mintAssetB, offeror.publicKey)
    const ataOffereeAssetA = await getAssociatedTokenAddress(mintAssetA, offeree.publicKey)
    const ataOffereeAssetB = await getAssociatedTokenAddress(mintAssetB, offeree.publicKey)

    console.log('Found 4 ATAs')

    return {
      offerorPdaState,
      offerorPdaBump,
      offereePdaState,
      offereePdaBump,
      mintAssetA,
      mintAssetB,
      ataOfferorAssetA,
      ataOfferorAssetB,
      ataOffereeAssetA,
      ataOffereeAssetB,
      swapState,
      swapBump,
      escrowState,
      escrowStateBump,
      escrowAta,
      escrowAtaBump
    }
  }

  it("can initiate, cancel and accept swap", async () => {
    const {
      offerorPdaState,
      offerorPdaBump,
      offereePdaState,
      offereePdaBump,
      mintAssetA,
      mintAssetB,
      ataOfferorAssetA,
      ataOfferorAssetB,
      ataOffereeAssetA,
      ataOffereeAssetB,
      swapState,
      swapBump,
      escrowState,
      escrowStateBump,
      escrowAta,
      escrowAtaBump
    } = await prepareForTest()

    const logUsersState = async () => {
      const [offerorInitialized, offereeInitialized] = await Promise.all([
        program.account.userState.fetch(offerorPdaState),
        program.account.userState.fetch(offereePdaState)
      ])
      console.log(offerorInitialized)
      console.log(offereeInitialized)
    }

    try {
      try {
        await program.methods.initializeUserState(offerorPdaBump)
        .accounts({
          userState: offerorPdaState,
          user: offeror.publicKey,
          userSeed: offeror.publicKey
        })
        .signers([offeror])
        .rpc()
      } catch (error) {
        console.log(`User ${offeror.publicKey.toString()} already initialized`)
      }
      try {
        await program.methods.initializeUserState(offereePdaBump)
        .accounts({
          userState: offereePdaState,
          user: offeror.publicKey,
          userSeed: offeree.publicKey
        })
        .signers([offeror])
        .rpc()
      } catch (error) {
        console.log(`User ${offeree.publicKey.toString()} already initialized`)
      }
  
      try {
        await program.methods.initializeSwapState(swapBump)
          .accounts({
            swapState,
            offeror: offeror.publicKey,
            offeree: offeree.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY
          })
          .signers([offeror])
          .rpc()
      } catch (error) {
        console.log('Swap state already initialized')
      }

      try {
        await program.methods.initializeEscrowState(escrowStateBump)
          .accounts({
            escrowState,
            mint: mintAssetA,
            offeror: offeror.publicKey,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY
          })
          .signers([offeror])
          .rpc()
      } catch (error) {
        console.log('Escrow State already initialized')
      }
  
      try {
        await program.methods.initializeEscrow(escrowAtaBump)
          .accounts({
            swapState,
            escrowState,
            escrow: escrowAta,
            mint: mintAssetA,
            ataOfferor: ataOfferorAssetA,
            offeror: offeror.publicKey,
            offeree: offeree.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY
          })
          .signers([offeror])
          .rpc()
      } catch (error) {
        console.log('Escrow ATA already initialized')
        console.log(error)
      }

      console.log('Escrow state PDA: ', escrowState.toString())

      await program.methods.addMintOfferee()
        .accounts({
          swapState,
          mint: mintAssetB,
          offeror: offeror.publicKey,
          offeree: offeree.publicKey
        })
        .signers([offeror])
        .rpc()
  
      const initiateSwapTxn = await program.methods.initiateSwap()
        .accounts({
          swapState,
          offeror: offeror.publicKey,
          offeree: offeree.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
  
          offerorState: offerorPdaState,
          offereeState: offereePdaState
        })
        .signers([offeror])
        .rpc()
      console.log(`sent asset A from offeror to escrow: ${initiateSwapTxn}`)
      await logUsersState()
      const swapStateInitialized = await program.account.swapState.fetch(swapState)
      console.log(swapStateInitialized)
      console.log(`Offeror: ${swapStateInitialized.offeror.toString()}`)

      await program.methods.closeEscrow()
        .accounts({
          swapState,
          escrowState,
          escrow: escrowAta,
          mint: mintAssetA,
          ata: ataOfferorAssetA,
          offeror: offeror.publicKey,
          offeree: offeree.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .rpc()
      console.log('closed escrow')
      
      await program.methods.cancelSwap()
        .accounts({
          swapState,
          offeror: offeror.publicKey,
          offeree: offeree.publicKey,
  
          offerorState: offerorPdaState,
          offereeState: offereePdaState
        })
        .signers([offeror])
        .rpc()
      console.log('Canceled swap')
      await logUsersState()

      // await program.methods.closeEscrow()
      //   .accounts({
      //     swapState,
      //     escrowState,
      //     escrow: escrowAta,
      //     mint: mintAssetA,
      //     ata: ataOffereeAssetA,
      //     offeror: offeror.publicKey,
      //     offeree: offeree.publicKey,
      //     tokenProgram: TOKEN_PROGRAM_ID
      //   })
      //   .rpc()
      // console.log('closed escrow')

      // await program.methods.transferNftFromOffereeToOfferor()
      //   .accounts({
      //     swapState,
      //     mint: mintAssetB,
      //     offeror: offeror.publicKey,
      //     offeree: offeree.publicKey,
      //     ataOfferor: ataOfferorAssetB,
      //     ataOfferee: ataOffereeAssetB,
      //     tokenProgram: TOKEN_PROGRAM_ID
      //   })
      //   .signers([offeree])
      //   .rpc()
      // console.log('transfered from offeree to offeror')

      // await program.methods.acceptSwap()
      //   .accounts({
      //     swapState,
      //     offerorState: offerorPdaState,
      //     offereeState: offereePdaState,
      //     offeror: offeror.publicKey,
      //     offeree: offeree.publicKey
      //   })
      //   .rpc()
      // console.log('accepted swap')
  
      // await logUsersState()
      console.log(await program.account.swapState.fetch(swapState))
    } catch (error) {
      console.error(error)
    }
  });
});
