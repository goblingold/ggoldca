import * as wh from "@orca-so/whirlpools-sdk";
import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import { Decimal } from "decimal.js";
import { Ggoldca } from "../target/types/ggoldca";

const POOL_ID = new anchor.web3.PublicKey(
  "Fvtf8VCjnkqbETA6KtyHYqHm26ut6w184Jqm4MQjPvv7"
);

const TOKEN_A_MINT_PUBKEY = new anchor.web3.PublicKey(
  "USDH1SM1ojwWUga67PGrgFWUHibbjqMvuMaDkRJTgkX"
);

const TOKEN_B_MINT_PUBKEY = new anchor.web3.PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

const DAO_TREASURY_PUBKEY = new anchor.web3.PublicKey(
  "8XhNoDjjNoLP5Rys1pBJKGdE8acEC1HJsWGkfkMt6JP1"
);

const CONFIRM_OPTS: anchor.web3.ConfirmOptions = {
  skipPreflight: true,
};

describe("ggoldca", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Ggoldca as Program<Ggoldca>;
  const userSigner = program.provider.wallet.publicKey;

  const whClient = wh.buildWhirlpoolClient(
    wh.WhirlpoolContext.withProvider(
      program.provider,
      wh.ORCA_WHIRLPOOL_PROGRAM_ID
    ),
    new wh.AccountFetcher(program.provider.connection)
  );

  const [vaultAccount, bumpVault] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        TOKEN_A_MINT_PUBKEY.toBuffer(),
        TOKEN_B_MINT_PUBKEY.toBuffer(),
      ],
      program.programId
    );

  console.log(vaultAccount.toString());
  console.log(program.programId.toString());

  const [vaultLpTokenMintPubkey, bumpLp] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint"), vaultAccount.toBuffer()],
      program.programId
    );

  it("Initialize vault", async () => {
    const vaultInputTokenAAccount = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      vaultAccount,
      true
    );

    const vaultInputTokenBAccount = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      vaultAccount,
      true
    );

    const daoTreasuryLpTokenAccount = await getAssociatedTokenAddress(
      vaultLpTokenMintPubkey,
      DAO_TREASURY_PUBKEY,
      false
    );

    const tx = await program.methods
      .initializeVault(bumpVault, bumpLp)
      .accounts({
        userSigner,
        inputTokenAMintAddress: TOKEN_A_MINT_PUBKEY,
        inputTokenBMintAddress: TOKEN_B_MINT_PUBKEY,
        vaultAccount,
        vaultInputTokenAAccount,
        vaultInputTokenBAccount,
        vaultLpTokenMintPubkey,
        daoTreasuryLpTokenAccount,
        daoTreasuryOwner: DAO_TREASURY_PUBKEY,
      })
      .rpc(CONFIRM_OPTS);
    console.log("initialize_vault", tx);
  });

  it("Open position", async () => {
    const pool = await whClient.getPool(POOL_ID);

    // Load everything that you need
    const poolData = pool.getData();
    const poolTokenAInfo = pool.getTokenAInfo();
    const poolTokenBInfo = pool.getTokenBInfo();

    // Derive the tick-indices based on a human-readable price
    const tokenADecimal = poolTokenAInfo.decimals;
    const tokenBDecimal = poolTokenBInfo.decimals;
    const tickLower = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(98),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );
    const tickUpper = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(150),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );

    const positionMintKeypair = anchor.web3.Keypair.generate();
    const positionPda = wh.PDAUtil.getPosition(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      positionMintKeypair.publicKey
    );
    const positionTokenAccountAddress = await getAssociatedTokenAddress(
      positionMintKeypair.publicKey,
      vaultAccount,
      true
    );

    const tx = await program.methods
      .openPosition(positionPda.bump, tickLower, tickUpper)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position: positionPda.publicKey,
        positionMint: positionMintKeypair.publicKey,
        positionTokenAccount: positionTokenAccountAddress,
        whirlpool: POOL_ID,
      })
      .transaction();

    const txSig = await program.provider.sendAndConfirm(
      tx,
      [positionMintKeypair],
      CONFIRM_OPTS
    );
    console.log("open_position", txSig);
  });
});
