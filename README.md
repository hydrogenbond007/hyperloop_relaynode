## TODO
1. Add batching feature (Funcion is present just need run it every interval)
2. Figure out how to extract block.timestamp and conversion rate from emitted data since they are not indexed

## How to run:
1. Set .env file with mentioned variables
2. Use updateCommittee in BridgeRx to set committee members
3. Run script file to call postMessage on BridgeTx cause it takes an array of custom type as argument.

## .env
```
DEST_CHAIN_ID=31337
SOURCE_CONTRACT_ADDR=0x95bD8D42f30351685e96C62EDdc0d0613bf9a87A
DEST_CONTRACT_ADDR=0x98eDDadCfde04dC22a0e62119617e74a6Bc77313
SOURCE_RPC_URL=ws://127.0.0.1:8545 //Web socket
DEST_RPC_URL=http://127.0.0.1:8545
PRIVATE_KEY=47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a
PUBLIC_KEY=ED1ECF0256AC7BD80A1D390641FC811218A32A99E0AFC40A534261168C78918E
```