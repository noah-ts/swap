import * as anchor from "@project-serum/anchor";
import { AnchorProvider, Program } from "@project-serum/anchor";
import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { clusterApiUrl, Connection, Keypair, PublicKey, sendAndConfirmTransaction, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } from '@solana/web3.js';
import { BslSwap } from "../target/types/bsl_swap";
import idl from '../target/idl/bsl_swap.json'

const programId = new PublicKey('EMxA8GoXaq4hQho3HeJNS2xLRwu5oesQXKF9BpskBA9m')

// this test tests on devnet, not local cluster
// private keys of offeror and offeree
const offerorPrivateKey = ''
const offereePrivateKey = ''

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
  const provider = new AnchorProvider(connection, offerorWallet, { preflightCommitment: 'processed' })
  const program = new Program(idl as any, programId, provider) as Program<BslSwap>

  const prepareForTest = async () => {
    const mintAssetA = new PublicKey('GtTMuni1CE4stN5uQBRv2tcKQaQrs1Z2uDSM3fBxL2T6')
    const mintAssetB = new PublicKey('E9kyTxayTbR9GK42Q7Ub1i4UzBmQeN6EpTaKymfAAsi6')

    const [offerorPdaState, offerorPdaBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('user_state'), offeror.publicKey.toBuffer()], program.programId
    )
    const [offereePdaState, offereePdaBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('user_state'), offeree.publicKey.toBuffer()], program.programId
    )
    const [swapState, swapBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('swap_state'), offeror.publicKey.toBuffer(), offeree.publicKey.toBuffer()], program.programId
    )
    const [escrowState, escrowBump] = await PublicKey.findProgramAddress(
    [anchor.utils.bytes.utf8.encode('escrow'), offeror.publicKey.toBuffer(), mintAssetA.toBuffer()], program.programId
    )
    console.log('Found 4 PDAs')

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
      escrowBump
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
      escrowBump
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
        await program.methods.initializeEscrow(escrowBump)
          .accounts({
            swapState,
            escrow: escrowState,
            mintAssetA,
            offeror: offeror.publicKey,
            offeree: offeree.publicKey,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY
          })
          .signers([offeror])
          .rpc()
      } catch (error) {
        console.log('Escrow already initialized')
      }
  
      const initiateSwapTxn = await program.methods.initiateSwap()
        .accounts({
          swapState,
          escrow: escrowState,
          mintAssetA,
          offeror: offeror.publicKey,
          offeree: offeree.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
  
          mintAssetB,
          ataOfferorAssetA,
          offerorState: offerorPdaState,
          offereeState: offereePdaState
        })
        .signers([offeror])
        .rpc()
      console.log(`sent asset A from offeror to escrow: ${initiateSwapTxn}`)
      await logUsersState()
      const swapStateInitialized = await program.account.swapState.fetch(swapState)
      console.log(swapStateInitialized)
      console.log(`Escrow: ${swapStateInitialized.escrow.toString()}`)
      console.log(`Offeror: ${swapStateInitialized.offeror.toString()}`)
  
      // await program.methods.cancelSwap()
      //   .accounts({
      //     swapState,
      //     escrow: escrowState,
      //     mintAssetA,
      //     offeror: offeror.publicKey,
      //     offeree: offeree.publicKey,
      //     tokenProgram: TOKEN_PROGRAM_ID,
  
      //     mintAssetB,
      //     ataOfferorAssetA,
      //     offerorState: offerorPdaState,
      //     offereeState: offereePdaState
      //   })
      //   .signers([offeror])
      //   .rpc()
      // console.log('Canceled swap')
      // await logUsersState()
  
      const transaction = new Transaction()
      transaction.add(
        await program.methods.acceptSwapOne()
          .accounts({
            swapState,
            escrow: escrowState,
            mintAssetA,
            offeror: offeror.publicKey,
            offeree: offeree.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
  
            mintAssetB,
            offerorState: offerorPdaState,
            offereeState: offereePdaState,
            ataOffereeAssetA
          })
          .instruction()
      )
      transaction.add(
        await program.methods.acceptSwapTwo()
          .accounts({
            swapState,
            mintAssetA,
            offeror: offeror.publicKey,
            offeree: offeree.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
  
            mintAssetB,
            ataOffereeAssetB,
            ataOfferorAssetB
          })
          .instruction()
      )
  
      const txn = await sendAndConfirmTransaction(connection, transaction, [offeree])
      console.log(`Accepted swap: ${txn}`)
      await logUsersState()
    } catch (error) {
      console.error(error)
    }
  });
});
