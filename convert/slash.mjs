import {Account, sequelize} from "./lib/models.mjs";
import {cryptoWaitReady} from "@polkadot/util-crypto";
import keyring from "@polkadot/ui-keyring";
import {one_p3d, provider, ss58Format} from "./lib/config.mjs";
import {ApiPromise} from "@polkadot/api";
import {Op} from "sequelize";

const limit = 100; // max accounts to slash per transaction
const mnemonic = "";

async function main() {
  await cryptoWaitReady();
  await keyring.loadAll({ss58Format, type: "sr25519"});

  const pair = keyring.createFromUri(mnemonic);
  console.log(`🔑 ${pair.address}`);

  const api = await ApiPromise.create({provider});

  await sequelize.sync();
  const accounts = await Account.findAll({
    where: {
      is_slashed: false,
      toSlash: {
        [Op.gt]: 0,
      },
    },
    order: [["toSlash", "DESC"]],
  });

  const accounts_count = accounts.length;
  console.log(`🔎 ${accounts_count} accounts to slash`);

  const slash_args = [];

  let count = 0;
  for (const account of accounts) {
    console.log(`https://3dpscan.io/account/${account.address} ➡️  ${BigInt(account.toSlash) / one_p3d}`);
    slash_args.push([account.address, BigInt(account.toSlash)]);
    if (++count >= limit) {
      break;
    }
  }

  const tx = api.tx.validatorSet.slashAccounts(slash_args);
  const preimage = api.tx.preimage.notePreimage(tx.method.toHex());
  const preimage_hash = await preimage.signAndSend(pair);
  console.log("✅ Preimage created", preimage_hash.toHex());

  for (const arg of slash_args) {
    await Account.update({is_slashed: true}, {
      where: {address: arg[0]}
    });
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(-1);
  });
