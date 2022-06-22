import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import { Ggoldca } from "../target/types/ggoldca";

const POOL_ADDRESS = new anchor.web3.PublicKey(
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

  it("Initialize vault", async () => {
    const [vaultAccount, _bump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("vault"),
        TOKEN_A_MINT_PUBKEY.toBuffer(),
        TOKEN_B_MINT_PUBKEY.toBuffer(),
      ],
      program.programId
    );

    const [vaultLpTokenMintPubkey, _bump2] =
      anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("mint"), vaultAccount.toBuffer()],
        program.programId
      );

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
      .initializeVault()
      .accounts({
        userSigner: program.provider.wallet.publicKey,
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
    console.log("Initialize vault", tx);
  });
});
