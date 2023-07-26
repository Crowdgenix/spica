import fs from 'fs';
import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import type { Hash } from '@polkadot/types/interfaces/runtime';
import { Abi } from '@polkadot/api-contract';
import Token_factory from '../types/constructors/token_factory';
import Token from '../types/contracts/token';
import 'dotenv/config';
import '@polkadot/api-augment';

// Create a new instance of contract
const wsProvider = new WsProvider('wss://rpc.shibuya.astar.network');
// Create a keyring instance
const keyring = new Keyring({ type: 'sr25519' });

async function main(): Promise<void> {
  const api = await ApiPromise.create({ provider: wsProvider });
  const deployer = keyring.addFromUri(process.env.PRIVATE_KEY || '');
  const tokenFactory = new Token_factory(api, deployer);
  const totalSupply = parseUnits(1_000_000).toString();
  const tokenContractRaw = JSON.parse(
    fs.readFileSync(__dirname + `/../artifacts/token.contract`, 'utf8'),
  );
  const tokenAbi = new Abi(tokenContractRaw);
  // let { gasRequired } = await api.call.contractsApi.instantiate(
  //   deployer.address,
  //   0,
  //   null,
  //   null,
  //   { Upload: tokenAbi.info.source.wasm },
  //   tokenAbi.constructors[0].toU8a([totalSupply, 'Apollo Token', 'APLO', 18]),
  //   '',
  // );
  const gasLimit: any = api.registry.createType('WeightV2', {
    refTime: BigInt(10000000000),
    proofSize: BigInt(10000000000),
  })
  const { address: aploAddress } = await tokenFactory.new(
    totalSupply,
    'Apollo Token' as unknown as string[],
    'APLO' as unknown as string[],
    18,
    // { gasLimit: gasLimit },
  );
  console.log('aplo token address:', aploAddress);
  const aplo = new Token(aploAddress, deployer, api);
  const totalSupplyQueryRes = await aplo.query.totalSupply();
  console.log("totalSupplyQueryRes", totalSupplyQueryRes)
  await api.disconnect();
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

function parseUnits(amount: bigint | number, decimals = 18): bigint {
  return BigInt(amount) * BigInt(10) ** BigInt(decimals);
}
