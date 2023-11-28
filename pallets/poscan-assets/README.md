# poscanAssets Module

A module for dealing with fungible and non-fungible assets issued as either independent currencies (tokens) or backed tokens (currencies backed by the object, according to [3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) tokenization standard rules). For example, the tokenization of the object properties, such as: share (%), grams, kilograms, square meters, etc. 
The module represents a modification of the [Assets](https://github.com/paritytech/substrate/tree/master/frame/assets) pallet provided by Substrate, so its API is quite similar. 

## Overview

The poscanAssets module provides such options as:

* Asset Issuance
* Asset Transfer
* Asset Destruction

### Terminology

* **Asset issuance:** The creation of a new asset, which total supply will belong to the
  account that issues the asset.
* **Asset transfer:** The action of transferring assets from one account to another.
* **Asset destruction:** The process of an account removing its entire holding of an asset.
* **Fungible asset:** An asset, the units of which are interchangeable with one another.
* **Non-fungible asset:** An asset representing one indivisible and unique unit. In th is particular module it stands for issuance an indivisible unit tethered to the object approved by the authentication protocol [3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md). ERC-721 is not supported. 

### Vision

The `poscanAssets` module allows for:

* Issuance of a unique asset and tether (or not to tether) it to the object previously approved by The Ledger of Things ([poScan](https://github.com/3Dpass/3DP/tree/main/pallets/poscan) module is being leveraged for the object authentication). The asset is possible to get tethered to one of the object properties like "non-fungible", "share", "grams", etc. And only one property must be chosen for the tokenization. It is prohibited to have the object tokenized twice simultaneously (ex. you cannot get tokenized both the object share and its weight, you have to pick up one). The object properties are managed by [poScan](https://github.com/3Dpass/3DP/tree/main/pallets/poscan) module, as well.
* Moving assets among accounts.
* The assets management - collective ownership, decision making process, etc. 

## Interface

### Dispatchable Functions

* `issue` - Issues the total supply of a new fungible asset to the account of the caller of the function.
* `transfer` - Transfers an `amount` of units of fungible asset `id` from the balance of
the function caller's account (`origin`) to a `target` account.
* `destroy` - Destroys the entire holding of a fungible asset `id` associated with the account
that called the function.

Please refer to the [`Call`](https://docs.rs/pallet-assets/latest/pallet_assets/enum.Call.html) enum and its associated variants for documentation on each function.

### Public Functions
<!-- Original author of descriptions: @gavofyork -->

* `balance` - Get the asset `id` balance of `who`.
* `total_supply` - Get the total supply of an asset `id`.

Please refer to the [`Pallet`](https://docs.rs/pallet-assets/latest/pallet_assets/pallet/struct.Pallet.html) struct for details on publicly available functions.

License: Apache-2.0
