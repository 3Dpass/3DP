import {ApiPromise} from "@polkadot/api";
import {Account, sequelize} from "./lib/models.mjs";
import cliProgress from "cli-progress";
import {one_p3d, provider} from "./lib/config.mjs";
import {bigIntMin} from "./lib/utils.mjs";

const miners = {};
const exchanges = [
  "d1HpAaVx9a4qoxmJbKZthjq9SL9L4Xukh2NNZtUHaXL5o4FLT", // Xeggex
  "d1D8KVqMFnYPLVVCpe59QVQZcW6fr5F6jU3d7wpkwyAiZmED2", // Txbit
  "d1J1WymQy1aVqstxWdY7wE6V1RNFtHkK68g3KKW1Sc3rUmBVF", // Caldera
  "d1Gc8Z4gu2jRaEYdRnCKeU2PWebH9G3Nv2fxYBH3KEUJmRZUL", // approved validator
];

async function main() {
  const api = await ApiPromise.create({provider});

  const [chain, nodeName, nodeVersion] = await Promise.all([
    api.rpc.system.chain(),
    api.rpc.system.name(),
    api.rpc.system.version(),
  ]);
  console.log(`You are connected to chain ${chain} using ${nodeName} v${nodeVersion}`);

  const minBlock = 280200;
  const maxBlock = 370898;

  const bar = new cliProgress.SingleBar({}, cliProgress.Presets.shades_classic);
  bar.start(maxBlock - minBlock, 0);

  let blockNumber = minBlock;
  while (blockNumber <= maxBlock) {
    const hash = await api.rpc.chain.getBlockHash(blockNumber++);

    const events = await api.query.system.events.at(hash.toString());
    events.forEach((record) => {
      const {event, phase} = record;
      if (event.section === "balances" && event.method === "Deposit") {
        const [who, value] = event.data;
        // skip validators shares
        if (value <= one_p3d * 50n) {
          return;
        }
        if (miners[who] === undefined) {
          miners[who] = BigInt(0);
        }
        miners[who] += BigInt(value);
      }
      if (event.section === "balances" && event.method === "Transfer") {
        const [from, to, value] = event.data;
        // track only transactions of mined P3D
        if (miners[from] === undefined) {
          return;
        }
        if (miners[to] === undefined) {
          miners[to] = BigInt(0);
        }
        miners[from] -= BigInt(value);
        miners[to] += BigInt(value);
      }
    });

    bar.update(blockNumber - minBlock);
  }

  await sequelize.sync();
  await Account.destroy({where: {}});

  for (const account of Object.entries(miners)) {
    if (exchanges.includes(account[0])) {
      continue;
    }

    const api_account = await api.query.system.account(account[0]);
    await Account.create({
      address: account[0],
      amount: account[1],
      free: api_account.data.free.toBigInt(),
      miscFrozen: api_account.data.miscFrozen.toBigInt(),
      toSlash: bigIntMin(account[1], api_account.data.miscFrozen.toBigInt()),
    })
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(-1);
  });
