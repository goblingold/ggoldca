//#! anchor test
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

const CONFIRM_OPTS_FIN: anchor.web3.ConfirmOptions = {
  skipPreflight: true,
  commitment: "finalized",
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

  let vaultInputTokenAAccount;
  let vaultInputTokenBAccount;

  const [vaultAccount, bumpVault] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        TOKEN_A_MINT_PUBKEY.toBuffer(),
        TOKEN_B_MINT_PUBKEY.toBuffer(),
      ],
      program.programId
    );

  const [vaultLpTokenMintPubkey, bumpLp] =
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("mint"), vaultAccount.toBuffer()],
      program.programId
    );

  it("Initialize vault", async () => {
    vaultInputTokenAAccount = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      vaultAccount,
      true
    );

    vaultInputTokenBAccount = await getAssociatedTokenAddress(
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

  let position;
  let positionTokenAccount;
  let tickArrayLowerPubkey;
  let tickArrayUpperPubkey;

  it("Open position", async () => {
    const pool = await whClient.getPool(POOL_ID);
    const poolData = pool.getData();
    const poolTokenAInfo = pool.getTokenAInfo();
    const poolTokenBInfo = pool.getTokenBInfo();

    const tokenADecimal = poolTokenAInfo.decimals;
    const tokenBDecimal = poolTokenBInfo.decimals;
    const tickLower = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(0.9),
        tokenADecimal,
        tokenBDecimal
      ),
      poolData.tickSpacing
    );
    const tickUpper = wh.TickUtil.getInitializableTickIndex(
      wh.PriceMath.priceToTickIndex(
        new Decimal(1.1),
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

    position = positionPda.publicKey;
    positionTokenAccount = positionTokenAccountAddress;

    const tx = await program.methods
      .openPosition(positionPda.bump, tickLower, tickUpper)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        position,
        positionMint: positionMintKeypair.publicKey,
        positionTokenAccount,
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

  it("Init tick arrays", async () => {
    const positionData = await whClient.fetcher.getPosition(position);
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    // Construct Init Tick Array Ix
    const tickArrayLower = wh.TickUtil.getStartTickIndex(
      positionData.tickLowerIndex,
      poolData.tickSpacing
    );
    const tickArrayUpper = wh.TickUtil.getStartTickIndex(
      positionData.tickUpperIndex,
      poolData.tickSpacing
    );

    const tickArrayLowerPda = wh.PDAUtil.getTickArray(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID,
      tickArrayLower
    );

    const tickArrayUpperPda = wh.PDAUtil.getTickArray(
      wh.ORCA_WHIRLPOOL_PROGRAM_ID,
      POOL_ID,
      tickArrayUpper
    );

    const initTickLowerIx = wh.WhirlpoolIx.initTickArrayIx(
      whClient.ctx.program,
      {
        startTick: tickArrayLower,
        tickArrayPda: tickArrayLowerPda,
        whirlpool: POOL_ID,
        funder: userSigner,
      }
    );

    const initTickUpperIx = wh.WhirlpoolIx.initTickArrayIx(
      whClient.ctx.program,
      {
        startTick: tickArrayUpper,
        tickArrayPda: tickArrayUpperPda,
        whirlpool: POOL_ID,
        funder: userSigner,
      }
    );

    tickArrayLowerPubkey = tickArrayLowerPda.publicKey;
    tickArrayUpperPubkey = tickArrayUpperPda.publicKey;

    const tx = new anchor.web3.Transaction()
      .add(initTickLowerIx)
      .add(initTickUpperIx);

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("init tick arrays", txSig);
  });

  it("Deposit", async () => {
    const poolData = await whClient.fetcher.getPool(POOL_ID);

    const liquidityAmount = new anchor.BN(1_000);
    const maxAmountA = new anchor.BN(10_000);
    const maxAmountB = new anchor.BN(10_000);

    const tokenOwnerAccountA = await getAssociatedTokenAddress(
      TOKEN_A_MINT_PUBKEY,
      userSigner
    );

    const tokenOwnerAccountB = await getAssociatedTokenAddress(
      TOKEN_B_MINT_PUBKEY,
      userSigner
    );

    const tx = await program.methods
      .deposit(liquidityAmount, maxAmountA, maxAmountB)
      .accounts({
        userSigner,
        vaultAccount,
        whirlpoolProgramId: wh.ORCA_WHIRLPOOL_PROGRAM_ID,
        whirlpool: POOL_ID,
        position,
        positionTokenAccount,
        tokenOwnerAccountA,
        tokenOwnerAccountB,
        tokenVaultA: poolData.tokenVaultA,
        tokenVaultB: poolData.tokenVaultB,
        tickArrayLower: tickArrayLowerPubkey,
        tickArrayUpper: tickArrayUpperPubkey,
      })
      .transaction();

    const txSig = await program.provider.sendAndConfirm(tx, [], CONFIRM_OPTS);
    console.log("deposit", txSig);
  });
});
