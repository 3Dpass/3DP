# Mining pool Pallet

This pallet is designed for Substrate based chains using a hybrid PoW + PoA (GRANDPA) consensus with vesting period applied for a while after having blocks mined.
For example, block rewards are staying locked for a month after its creation. 

The pallet allows for creation numbers of mining pools the block rewards of which are going to be distributed among miners directly from the network without a middlemen 
(as if they were to mine SOLO). This is guaranteed for miners and pool admins to get rewarded, irrespective to what kind of devices (CPU, GPU or whatever) they have connected with.

In order to prove the miner's work there is an additional off-chain difficulty number being leveraged by the client app [pass3d-pool](https://github.com/3Dpass/pass3d-pool) and verified on the pool Node's side. The additional 
difficulty is set up by the pool's admin. Every 10 sec the client app is requesting the pool Node (via the [RPC API](https://github.com/3Dpass/3DP/wiki/RPC-API-mining-pool-interaction)) for some necessary mining metadata.

There is a statistic report being saved by the pool Node every 20 blocks on the chain storage, which is available for everyone. Once the block is mined by the mining pool Node and accepted by the network, the block rewards are being 
distributed directly in proportion to the input hashrate provided by each miner in the pool.

## Monopoly restriction rule 
In order to ensure the network healthiness and prevent it from being taken over by just a few mining pool Nodes hosting the whole network, there is a restriction rule, which will incentivize to create 10 mining pools at minimum. 
Each pool should not exceed the limit = 10% of luck calculated statistically from the blockchain history. History depth for the pool_rate to calculate: 100 blocks back. 

There is no technical restrictions, but the economic penalties being applied to the minng block rewards, once having the limit rule broken up. The maximum penalties share possible to get is 100% mining block rewards, which is being 
slashed as the pool_rate has exceded the threshold of 2*limit (20% of luck). All the penalties are being slashed to the Treasury pot controlled by the Governance. 

### Penalties share conditions

```pool_rate <= limit
(pool_rate - limit) / limit
limit < pool_rate < 2*limit
pool_rate >= 2*limit
```

## Identity and KYC integration
Minimun Idenity level of confidence requirements: 
- Pool admin - `Reasonable`, which means that there is at least one contact (Twitter, Google, Discord, Telegram) proved to be associated with the user's network address and there is no duplicates found on the blockchain.
- Pool member - depends on the option chosen by the pool admin: 
  - `Anonymous` - there is no Idenity requirements for the pool members to join
  - `KYC` - Reasonable Identity level is required

### Selection threshold
Not only one Registrar could provide its Identity judgement on any address they have been requested for, but numbers of them available. And 2/3 positive judgements will be enough for the account to pass. 

### One Reasonable Identity = One pool admin
- It is allowed for only one unique Reasonable Identity to be a pool admin. All duplicates will be rejected.
- If the Idenity level was downgraded for a pool admin, the whole pool is geting Suspended, until it becomes Reasonable back again.

### One Reasonable Identity = One pool member
These rules will be applied to the Reasonable accounts only: 
- It is allowed for only one unique Reasonable Identity to be a pool member. Either, will duplicates be rejected or removed from every pool. For example, there are 4 duplicates detected among some different pools, then all of them are to be rulled out.
- If the Identity level was downgraded for a pool member, will this member be ruled out of the KYC pool they are in now.
- It is allowed for the pool member to have numbers of different mining devices connected to the pool at the time (using the same mining address as a member_id for different devices is allowed).

### One anonymous address = one anonymous pool member
- There is only one rule applied to anonymous pool members, which says that all addresses must be unique. For example, if someone would like to join several anonymous pools, they are going to have to create an address for each one. 

