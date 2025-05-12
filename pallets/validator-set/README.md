# Validator set
This pallet is to control the set of authorities eligible to vote for the blockchain finalization 
in accordance with the [GRANDPA](https://github.com/w3f/consensus/blob/master/pdf/grandpa.pdf) protocol. 
Additionally, the validators take part in users' object authentication in the [3DPRC2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) tokenization standard 
operating within The Ledger of Things (LoT). The Validator set is open to participate/exit for any Node 
under the [SLA](https://3dpass.org/mainnet#validator) conditions. 

## Incentive mechanism:
- Block finalizaton rewards: 50% of total block rewards (the rest 50% goes to the block author)
- The user objects processing: 50% of the object verification fee (paid by user)

## Selection threshold
There is a threshold for new validator to pass, which includes some certain amount of P3D locked 
up for the collateral as well as to prove the set up fee transaction paid to Treasury. The set up 
fee is required to pay just once. Hovewer, the collateral needs to remain locked up all the way 
through the node operating period. If the lock out period expires, the node is being moved out 
the validator set. In order to get back the threshold is required to pass again.

## Punishments
- Equivocation attack (Voting for two different chains simultaneously. It mostly occurs to the validators using the same keyset on different servers and thus causing a threat for the network to get split): 40 000 P3D and getting out of the validator set as well as from the session
- Not being online/available: 20 000 P3D and getting out of the validator set
- Not being able to participate in GRANDPA voting rounds for any reason (Firewall, incorrect keys set up, etc.): 20 000 P3D and getting out of the validator set
- Not being able to get the user object processed in 20 sec: 100% of the user object validation serivce fee and getting out of the validator set
- Losing Reasonable level of confidence for the on-chain identity: getting out of the validator set

## Rejoining the validator set
There is a "comeback window" in place to let validators rejoin the set free of charge. No set up fee will be charged throughout the period.
- Ban period: 3 hours since heading off the validator set
- Comeback window: ~ 1 week since the ban period got expired

## Legitimate exit
There is a legitimate way for Validators to exit without getting penalized, which is to wait until the lock period has expired and call out the `unlock` method.

## API
- `lock(amount, until, period)` - locks certain amount of P3D for the collateral. The `periood` is the auto re-lock option.   
- `addValidatorSelf()` -  adds a validator-candidate to the queue
- `rejoinValidator (validatorid)` - allows to rejoin the validator set within the "comback window" period
- `unlock(amount)` - unlocks certain amount of P3D is the lock period has expireed
- `payPenalty` - allows to pay off all the penalties set for the validator
- `submitEstimation (est, signature)` - system method leveraged by validators to submit the estimation on the user objects processing time (see more: [3DPRC2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md))
