# PoScan pallet (3DPRC-2 implementation)

[3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) (3Dpass Request for Comments), proposed by PaulS in September 2023, is a standard p2p protocol for the tokenization of objects operating within “The Ledger of Things” decentralized blockchain platform. Substrate based.

The PoScan pallet is integrated into 3dpass network runtime providing the access to the network decentralized storage by means of the object tokenization API, which allows for:
- the user object authentication and its protection from being copied to the extent for the recognition algorithm precision;
- non-fungible digital asset creation;
- property rights definition and its transfers;
- backed cryptocurrency issuance (fungible tokens backed by the asset).

## API

### 1. poscan.putObject

This method allows for users to put an object into the poScan storage. The object authentication procedure will be triggered, as well. Either, the object will be `Approved` or `NotApproved` as a result (see more THE OBJECT AUTHENTICATION PROTOCOL). 

- If `Approved`, the object will be allowed for any further operation with the asset (property rights transfers, backed currency issuance, etc), and the copy protection will be applied. The object will be available on the network storage with all the authentication history data attached.

- `NotApproved` keeps the object and all the authentication history data available on the network storage, however, all further operations will be prohibited, and the copy protection will not be applied.


```
putObject(
       category, 
       obj, 
       numApprovals,
       hashes,
       properties
)
```

`category` - the object category: 
- `Objects3D`: 
   - `Grid2dLow` - Grid2d algorithm (low precision), preset: `-s 12 -g 8 -a grid2d_v3a` (see more [pass3d](https://github.com/3Dpass/pass3d) recognition toolkit to learn about Grid2D algo parameters) ,
   - `Grid2dHigh` - Grid2d algorithm (high precision),
- `Drawings2D`, 
- `Music`, 
- `Biometrics`, 
- `Movements`, 
- `Texts`

`obj ` - the object to tokenize (ex. 3D model in .obj format)

`numApprovals: u8` - the number of confirmations in blocks to order (1-255). The object authentication loop will repeat itself as much times as it is requested

`hashes: Option<Vec<H256>>`  - the hashes (10 at max) of the object HASH ID (ex. the top10 hashes Grid2D output)

`properties: Vec<SpConsensusPoscanPropValue> (Vec<PropValue>)` - the list of the object properties, which might be used for its tokenization (Non-fungible, Share, Weight, Density, etc). Each property is defined by two parameters: 
- `propIdx: u32` - the property index on the `poScan` module storage (ex, 0 - Non-fungible; 1 - Share)
- `maxValue: u128` - Max Supply limit for tokens, backed by this property (might be issued afterwards, if having the object `Approved`). `maxValue: 1` is a must for the `propIdx: 0` (Non-fungible). A value in the format of `10^x` is a must for the `propIdx: 1`(Share). For example, `maxValue: 1000000` or `maxValue: 1000` are both correct, and the `maxValue: 2000000` or `maxValue: 1234567` are incorrect.   

### 2. poscan_getPoscanObject

The objects are getting compressed with zip, before they are put on the storage. This method allows to get the object and its history unzipped. 

```
curl -H "Content-Type: application/json" -d 
'{"id":1, 
"jsonrpc":"2.0",
"method": "poscan_getPoscanObject",
"params": [0]
}' http://localhost:9933/
```
- Where the `"params": [0]` is the object index value

The output provides `result` parameter containing the object itself and all the history related to:

```
{"jsonrpc":"2.0","result":
{"state":{
      "Approved":101
    },
"obj"[
111,32,10,118,32,48,46,48,57,32,48,46,56,50,32,45,48,46,48,49,10,118,32,48,46,48,56,32,48,46,56,49,32,48,46,48,51...
    ],
"compressed_with":null,
"category":
   {
     "Objects3D":"Grid2dLow"
   },
"hashes":[
"0x2c998a9919725d59e477fcb823f811e1ac5806722653906f5b5d7ec933ef6bdf",
"0x80af29c76bf63b639efd8fe8ce368a27266d538de0a47a39725bc0bb0e13a865",
"0xb46a7668a64c09b19ff9f42e9b68d78268f11715a35ca125ddf48bceee5097e4",
"0xd9d8dbf4258bc7de9c7de7ac5d5dbec806bcb04ca995fd133482390f047003bc",
"0x8d1b99511af0a0c6cb43a7546b75fd7d7a247cbd3e4a6259ac0b28cff67172a2",
"0x24aad89b485f3e84c4c37e1ba26355c577879123bb02fb010f576ac352ce88b1",
"0x0691088a9b80ca199efb27a06a043a35500de3807d55757590130547f5bb24f7",
"0xfaf0c5025b9573c0a06ce7f0c7e3737717005f92866f0f25b45b57cb400b90c2",
"0x76c10fcff1dfda6c3ee665396f61543a26bdd9f73bf4b656f116495aecc9f53b",
"0xe465088c87c5c8cf2f33436ee0152107439bdb97335920abeff19d1c3eb6abf7"
],
"when_created":91,
"when_approved":101,
"owner":"d7f5KGsoZ3xzB6Ecmv92DPizD1x5eToNHM1CPfSC1Xu4nzecN",
"estimators":[
     [
       "d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
       53
     ],
     [
       "d1asoD7V6hdff4ExvtjRbdVT398Rg8fKEruYsKFS5P9mkT4fy",
       5
     ]
   ],
"est_outliers":[],
"approvers":[ 
     {
       "account_id":"d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
       "when":96,
       "proof":"0x93b64823cb53e8c08b1ddf1b30bed611b11e0912b3b28aa3e122b2fb5d418be8"
     },      
     {
       "account_id":"d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
       "when":97,
       "proof":"0xcff4f0b718ee26934fa6a7739e2111d307e2e7be491047277160f92bd5dee1fb"
     },  
    {
       "account_id":"d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
       "when":98,
       "proof":"0xa0de6a64564f85e481fe8fbea5f8c6ddf13b3e7e66c00254f27a290407d1d99b"
     },
    {
       "account_id":"d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
       "when":99,
       "proof":"0x961d9110c11ceb62c447edb8576fc8721550a829814e8f69e1732f5e101e5dd4"
    },
    {
       "account_id":"d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
       "when":100,
       "proof":"0x9dec30de83339dffa02093cf5c2d3e110c47e2f9dc05a237f73af8cae6b89b0f"
    },
    {
"account_id":"d1ePg4fK97U913xAnZzZ9dUf1W1XUA4bYVC2Zre8K9vjSixnc",
"when":101,
"proof":"0x145f0fd64c2865e05f5a727d6dc316ac6164111a785b5bba8ff4fddec1c03171"
    }
],
"num_approvals":6,
"est_rewards":70000000000000000,
"author_rewards":30000000000000000
"prop": [
        {
          "propIdx": 0,
          "maxValue": 1
        }
      ]
}
```

Whereas the following parameters supplied:

- `state: {Status: blockN}` - current status of the authentication process (ex. Estimated: 2,693 - the block at which the estimation finished)
- `obj:` - the object submitted by user (ex. 3D model in .obj format)
- `category: {Objects3D: Grid2dLow}` - the object category and the algorithm preset used for its authentication
- `hashes:[]` - the HASH ID submitted by user (ex. the top 10 hashes from Grid2d output)
- `whenCreated: blockN` - the block number the object was created at
- `whenApproved: blockN ` - the block number the object was approved at
- `owner: P3D address` - the object owner (initially, this is a P3D account the object was submitted with)
- `estimators: [P3D address, time in mSec]` - the list of Validators (P3D addresses), who voted for the object to pass the estimation threshold (2/3 validators out of the number of validators in the set), the object processing time provided
- `estOutliers:[P3D address, time in mSec]` - the validators ruled out of vote, due to the weird processing time
- `approvers:[{"account_id":P3D address,"when":blockN, "proof":hash}]` - the list of block authors (miners) provided their judgement on the object authenticity. `"proof":hash` - is a "zero knowledge proof" hash (ex. the 11th hash of Grid2D output in sha256) 
- `numApprovals: N` - the number of confirmations ordered by user
- `estRewards: 70,000,000,000,000,000` - the validator share of rewards in minimum indivisible units “Crumbs”, 1 Crumb = 0.0000000000001 P3D), which is to be distributed among the validators(estimators)
- `authorRewards: 30,000,000,000,000,000` - the block author share of rewards in minimun indivisible units “Crumbs”, which is to be distributed among the the miners (approvers)
- `prop: [{"propIdx": u32, "maxValue": u128}]` - the list of the object properties (inherent to the object).
  - `propIdx: u32` - the property index on the `poScan` module storage (ex, `0` - Non-fungible; `1` - Share)
  - `maxValue: u128` - Max Supply limit for tokens, backed by this property (might be issued, if having the object `Approved`).

### 3. poscan.setAlgoTime

This method allows to set up the time frame limit in seconds for the objets to get processed. This value can be set up by the Council vote only.

```
setAlgoTime(
           algoTime: u32
)

```

- `algoTime: u32` - the timeframe limit in seconds (10 seconds is set up by default)

### 4. poscan.setFeePerByte

This method allows to set up the object authentication fee. This value can be set up by the Council vote only.

```
setFeePerByte(
            fee: u64
)

```
- `fee: u64` - P3D/Byte for user to pay

### 5. poscan.approve

This method is utilized by new block authors (miners) to provide their judgement on objects “Estimated” (after the estimation procedure, performed by Validators, is completed successfully). If the estimation procedure was not successful or fully complete (ex, the object is still at “Estimating” or “Created”), the judgement and the block will be rejected by the majority of the network.

```
approve(
        author, 
        objIdx, 
        proof
)
```

- `author: P3D address` - block author
- `objIdx: u32` - the object index on the poScan storage
- `proof: Option<H256>` - zero-knowledge proof of work hash (see more THE OBJECT AUTHENTICATION PROTOCOL)

### License
License: Unlicense

Copyright (C) 2023 - 3Dpass.org
