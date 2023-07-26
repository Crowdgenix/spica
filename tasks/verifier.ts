import { ApiPromise, WsProvider, Keyring } from '@polkadot/api'
import { ContractPromise } from '@polkadot/api-contract'
import { readJSON } from 'fs-extra'
import path = require('node:path')
import { stringToU8a, u8aToHex } from '@polkadot/util'
import { BN, BN_ONE } from '@polkadot/util'
import Ido from '../types/contracts/ido';

async function main() {
    const wsProvider = new WsProvider('wss://ws.test.azero.dev')
    const api = await ApiPromise.create({ provider: wsProvider })

    const keyring = new Keyring({ type: 'ecdsa' })
    const account = {
        address: '5CSvrxpAJeWheNmsJacLsU8Xmiim1osMP97d8Pux9rACE731',
        key:
            'give wine giant genre trim razor drill garment deposit pepper there choose//0',
    }
    const pair = keyring.addFromUri(account.key,)
    // const pair = keyring.addFromUri('//Alice')
    console.log('\x1b[36m%s\x1b[0m', 'pair', pair.address)

    const message =
        'buy_ido_ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff_d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d_1000000000000'
    const sig = Array.from(pair.sign(message))

    // Please paste contract address
    const contractAddress = '5EaUvA4RogcB9VEmXnLajDBjRBxCSDmoBfeLaXpwJ3MHENh1'
    const metadata = await readJSON(
        path.resolve(
            __dirname,
            '/Users/trancuong/Sotatek/CrowndGenix/crownd-genix-contract/contracts/ido/target/ink/ido.json',
        ),
    )
    // const contract = new ContractPromise(api, metadata, contractAddress)
    const idoInstance = new Ido(contractAddress, pair, api)

    const storageDepositLimit = null
    const gasLimit: any = api.registry.createType('WeightV2', {
        refTime: BigInt(10000000000),
        proofSize: BigInt(10000000000),
    })
    console.log("\x1b[36m%s\x1b[0m", "u8aToHex(pair.sign(message))", u8aToHex(pair.sign(message)));

    const queryResult = await idoInstance.query.verifySignature(Array.from(pair.sign(message)), message, {
        gasLimit,
    })

    console.log(`Query result: ${queryResult}`)
//   const MAX_CALL_WEIGHT = new BN(5_000_000_000).isub(BN_ONE)
//   const PROOFSIZE = new BN(1_000_000)

//   const tx = contract.tx['ido::buyIdoWithNative'](
//     {
//       // gasLimit,
//       gasLimit: api.registry.createType('WeightV2', {
//         refTime: MAX_CALL_WEIGHT,
//         proofSize: PROOFSIZE,
//       }) as any,
//       storageDepositLimit: '1',
//     },
//     sig,
//   )

//   const res = await tx.signAndSend(pair)
//   console.log('\x1b[36m%s\x1b[0m', 'res', res.toHuman())

    await api.disconnect()
}

main()
    .catch(console.error)
    .finally(() => process.exit())
