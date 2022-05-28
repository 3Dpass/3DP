[Proof of Scan](https://3dpass.org/proof_of_scan.html) is a PoW consensus based on 3D shape recognition technology. The network Nodes will stand guard preventing assets stored on the network from copy making, even if you change the file with one dot, pixel or 1 byte. The blockchain represents the Ledger of unique things.

In order to propose a new block Autor has to pick up a specific-shaped 3D object all the Nodes on the network can recognize and verify. By means of comparing their [HASH IDs](https://3dpass.org/features.html#3D_object_recognition), the Nodes can reject the same shaped objects. Because of the computing power needs for recognition as well as for picking up the objects, Authors are getting engaged enough to take Authority to vote for the longest chain and maintain the network, including users assets authenticity check.

- [p3d](https://github.com/3Dpass/p3d) tool is used for 3D objects recognition. It leverages [Grid2d](https://3dpass.org/grid2d.html) recognition algorithm. 
- On order to start mining run [Miner](https://github.com/3Dpass/miner), wich pushes random generated 3D models via [mining RPC](https://github.com/3Dpass/3DP/blob/dev/nodes/poscan-consensus/src/mining_rpc.rs) in the following format: 

```
{
    "jsonrpc":"2.0",
    "id":1",
    "method":"push_mining_object",
    "params": [
        1,
        "o\n
v 0.05508197844028473 0.7671535015106201 -0.14178061485290527\n
v 0.05349433422088623 0.764365017414093 -0.10946107655763626\n
v 0.04743874818086624 0.7608485817909241 -0.07884219288825989\n
            ]
}
```
Where as one of the parameters is the content of 3D model in .obj format, but with `\n` added at the end of each line. 
```
v 0.05508197844028473 0.7671535015106201 -0.14178061485290527\n
v 0.05349433422088623 0.764365017414093 -0.10946107655763626\n
v 0.04743874818086624 0.7608485817909241 -0.07884219288825989\n
```


