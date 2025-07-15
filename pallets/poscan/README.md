# PoScan pallet (3DPRC-2 implementation)

[3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) (3Dpass Request for Comments), proposed by PaulS in September 2023, is a standard p2p protocol for the authentication of objects operating within “The Ledger of Things”.

The PoScan pallet is integrated into 3dpass network runtime providing the access to the network decentralized storage by means of the object authentication API, which allows for:
- the user object authentication and its protection from being copied to the extent for the recognition algorithm precision;
- non-fungible digital asset creation;
- property rights definition and its transfers;

## API

### 1. poscan.putObject

This method allows for users to put an object onto the poScan storage, which triggers the object authentication procedure resulting as either `Approved` or `NotApproved`. (see more THE OBJECT AUTHENTICATION PROTOCOL). 

- If `Approved`, the object will be allowed for any further operation with the asset (property rights transfers, backed currency issuance, etc), and the copy protection will be applied. The object will be available on the network storage with all the authentication history data attached.

- `NotApproved` keeps the object and all the authentication history data available on the network storage, however, all further operations will be prohibited, and the copy protection will not be applied.

```
putObject(
    category,
    is_private,
    obj,
    numApprovals,
    hashes,
    properties,
    is_replica,
    original_obj,
    sn_hash,
    is_self_proved,
    proof_of_existence,
    ipfs_link
)
```

**Parameters:**
- `category`: The object category (e.g., `Objects3D:Grid2dLow`, `Objects3D:Grid2dHigh`, etc.).
- `is_private`: Boolean. If true, the object is private and only accessible to permitted users.
- `obj`: The object data to tokenize (e.g., a 3D model in .obj format), as a byte array.
- `numApprovals`: Number of block author approvals required (u8, 1-255).
- `hashes`: Optional. List of up to 10 object hashes (e.g., top 10 Grid2D output hashes).
- `properties`: List of object properties for tokenization. Each property is `{ propIdx: u32, maxValue: u128 }`.
- `is_replica`: Boolean. If true, this object is a replica of another.
- `original_obj`: Optional. The object index of the original object if this is a replica.
- `sn_hash`: Optional. Serial number hash for replicas (u64).
- `is_self_proved`: Boolean. If true, the object is self-proved and skips estimation/approval.
- `proof_of_existence`: Optional. Hash proving the object's existence (H256). Auto-calculated if not provided and `is_self_proved` is true.
- `ipfs_link`: Optional. IPFS link to the object or related data (byte array, max 512 bytes).

---

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

---

### 3. poscan.approve

This method is utilized by new block authors (miners) to provide their judgement on objects “Estimated” (after the estimation procedure, performed by Validators, is completed successfully). If the estimation procedure was not successful or fully complete (ex, the object is still at “Estimating” or “Created”), the judgement and the block will be rejected by the majority of the network.

```
approve(
    author, 
    objIdx, 
    proof
)
```
**Parameters:**
- `author`: AccountId of the block author (address).
- `objIdx`: Index of the object to approve (u32).
- `proof`: Optional. Zero-knowledge proof hash (H256).

---

### 6. poscan.inspectPutObject

Submit an object for QC (Quality Control) inspection, specifying an inspector, inspector fee, and custom timeout. The object enters a QC inspection state and must be approved or rejected by the inspector.

```
inspectPutObject(
    category,
    is_private,
    obj,
    numApprovals,
    hashes,
    properties,
    is_replica,
    original_obj,
    sn_hash,
    inspector,
    inspector_fee,
    qc_timeout,
    is_self_proved,
    proof_of_existence,
    ipfs_link
)
```
**Parameters:**
- All parameters as in `putObject`, plus:
- `inspector`: AccountId of the inspector (address).
- `inspector_fee`: Fee to be reserved and paid to the inspector (balance).
- `qc_timeout`: Custom timeout (in blocks) for the QC inspection phase.

---

### 7. poscan.qcApprove

Called by the inspector to approve an object under QC inspection. The inspector receives the reserved inspector fee.

```
qcApprove(
    objIdx
)
```
**Parameters:**
- `objIdx`: Index of the object under QC inspection (u32).

---

### 8. poscan.qcReject

Called by the inspector to reject an object under QC inspection. The inspector still receives the reserved inspector fee.

```
qcReject(
    objIdx
)
```
**Parameters:**
- `objIdx`: Index of the object under QC inspection (u32).

---

### 9. poscan.setPrivateObjectPermissions

Allows the owner of a private object to set or update permissions for who can create replicas of the object.

```
setPrivateObjectPermissions(
    objIdx,
    permissions
)
```
**Parameters:**
- `objIdx`: Index of the private object (u32).
- `permissions`: List of permissions (who, until, max_copies, etc.).

---

### 10. poscan.transferObjectOwnership

Transfers ownership of an object to another account, if allowed (e.g., no associated asset, not abdicated).

```
transferObjectOwnership(
    objIdx,
    newOwner
)
```
**Parameters:**
- `objIdx`: Index of the object to transfer (u32).
- `newOwner`: AccountId of the new owner (address).

---

### 11. poscan.abdicateTheObjOwnership

Allows the owner to irreversibly abdicate ownership of an object.

```
abdicateTheObjOwnership(
    objIdx
)
```
**Parameters:**
- `objIdx`: Index of the object to abdicate (u32).

---

### 12. poscan.setAlgoTime

Sets the maximum allowed time for object processing (Council/admin only).

```
setAlgoTime(
    algoTime
)
```
**Parameters:**
- `algoTime`: Maximum allowed time for object processing (u32, in seconds).

---

### 13. poscan.setFeePerByte

Sets the fee per byte for object authentication (Council/admin only).

```
setFeePerByte(
    fee
)
```
**Parameters:**
- `fee`: Fee per byte for object authentication (u64).

---

### 14. poscan.addPropertyType

Adds a new property type that can be associated with objects (admin only).

```
addPropertyType(
    name,
    class,
    maxValue
)
```
**Parameters:**
- `name`: Name of the property (string/byte array, max 64 bytes).
- `class`: Property class (e.g., Relative, Absolute).
- `maxValue`: Maximum value for the property (u128).

---

### 15. poscan.setDynamicRewardsGrowthRate

Sets the dynamic rewards growth rate for estimators (Council only).

```
setDynamicRewardsGrowthRate(
    growthRate
)
```
**Parameters:**
- `growthRate`: Dynamic rewards growth rate (u32, recommended 1-100).

---

### 16. poscan.setRewards

Sets the base rewards for object authentication (Council only).

```
setRewards(
    rewards
)
```
**Parameters:**
- `rewards`: Base rewards for object authentication (balance).

---

### 17. poscan.unlockUnspentRewards

Allows the fee payer to unlock and reclaim unspent rewards for a NotApproved object.

```
unlockUnspentRewards(
    objIdx
)
```
**Parameters:**
- `objIdx`: Index of the NotApproved object (u32).

---

### 18. poscan.setAuthorPart

Sets the percentage of rewards allocated to authors (Council only).

```
setAuthorPart(
    part
)
```
**Parameters:**
- `part`: Percentage of rewards allocated to authors (Percent type).

---

## Dynamic Rewards Formula

The PoScan pallet uses a dynamic rewards formula to incentivize validators based on the current queue size of objects waiting for processing. The formula increases rewards as the queue grows, encouraging faster processing during periods of high demand.

**Formula:**

```
dynamic_rewards = base_rewards * (1 + sqrt(queue_size) / growth_rate)
```

- `base_rewards`: The base reward amount (set by governance).
- `queue_size`: The number of objects currently in the queue (states: Created, Estimating, or QCInspecting).
- `growth_rate`: A configurable parameter (set by governance) that controls how quickly rewards increase with queue size. Higher values mean slower growth.

**Explanation:**
- If there are no objects in the queue, the dynamic rewards equal the base rewards.
- As the queue size increases, the rewards grow according to the square root of the queue size, divided by the growth rate.
- This mechanism helps balance validator incentives and network throughput.

### License
License: Unlicense

Copyright (C) 2023 - 3Dpass.org
