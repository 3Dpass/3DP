## Proof of Scan: PoW component
The implementation of a PoW (proof of work) component of the [Proof of Scan](https://3dpass.org/features#scanproof) hybrid consensus protocol based on 3D shape recognition technology. 
Nodes are capable of the objects authentication, which will prevent assets from being copied within p2p network, even if the *.obj* file, containing the object geometric data, has been modified with one dot, pixel or 1 byte. The nodes are equipped with the [pass3d](https://github.com/3Dpass/pass3d) recognition toolkit as well as its WASM version [p3d](https://github.com/3Dpass/p3d).

In The Ledger of Things (LoT) the PoW component is responsible for such aspects as:
 1.  New block producion
 2.  User's objects authentication in accordance with the [3DPRC2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) token standard

## ASIC/FPGA resistance
The Ledger of Things (LoT) Proof of Work (PoW) component is designed to resist ASIC/FPGA devices, ensuring a high level of mining distribution, which plays an essential role in the [3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) tokenizaton standard operating within the network. Leveraging 3D object shape as nonce makes [Proof of Scan](https://3dpass.org/proof-of-scan) unique and stand out from any other PoW around the crypto space. 

In order to find new block miners are picking up a unique-shaped 3D object, the [HASH ID](https://3dpass.org/features#recognition-hash-id) of which would make [Grid2d](https://3dpass.org/grid2d) recognition algorithm produce a specific output sealing the block on top of the Best chain.

## PoW task: 1 Node = 1 Vote, ASIC-resistant
LoT leverages an advanced version of PoW component secured with some additional factors, especially designed to facilitate on distributed mining and to ensure its resistance against ASIC/FPGA, such as: 

### One way calculation
The difficulty for the reverse (illegal) method  of calculation (x) is related to the difficulty for the straightforward (legit) method (m) as `x = m^n`. That will protect nonce from being recovered out of the `pre_hash` directly, which would let the block author skip 3D object shape recognition in the mining loop.

This is achieved by introducing the `rotation_bytes` (8 bytes defining the angle of 3D object rotation) positioning the object against the cross-section plane at a certain angle before getting cut. 

In order to prove 3D object existence at the beginning of the mining loop the bock author first must get 3D object scanned with no rotation angle `(μ = 0)` resulting as Grid2d output 1 (top 10 hashes). The second component is `time_hash`, which must timestamp upon the block header and impact the `pre_hash` , as well.

<img width="316" alt="no_rotation" src="https://github.com/user-attachments/assets/c1f2f049-4ef2-4e45-9517-b8292e2f93af" />

The `rotation_bytes` are calculated out of a combination of both the `pre_hash` and the topest Grid2d zero-angled hash (μ = 0) processed by the RandomX hashing. So that any modification made on either of them will inevitably impact on the rest.

The target rotation angle (μ = x) could only be calculated after having the object processed with zero angle (μ = 0). The same as usual the block author will be challenged with picking up some 3D object shape and getting it scanned with the target rotation angle (μ = x) until the `pre_hash` makes the double_hash meet current difficulty number or above.

<img width="325" alt="x_rotation-2" src="https://github.com/user-attachments/assets/518c8883-5253-430b-b3db-f420ca7c878b" />

### RandomX(Grid2d) output
[Grid2d](https://3dpass.org/grid2d) recognition algorithm output is getting processed by the [RandomX](https://github.com/tevador/RandomX) hashing function to equalize the mining velocity of CPU and GPU to one another.

<img width="529" alt="grid2d_randomXed" src="https://github.com/user-attachments/assets/aa81657b-34f6-455e-85ab-4289e8ad727d" />

The entire cycle of the object processing looks like this:

`Random 3D model in obj → Grid2d (3D model μ = 0) output → Random X (Grid2d μ = 0) blake2b hash → rotation_bytes x = 4 bytes out of pre_hash + 4 bytes out of RandomX (Grid2d μ = 0) blake2b hash → Grid2d (3D model μ = xo) output → Random X (Grid2d μ = x) blake2b hash`

### Proof of Context rule 
Leveraging full blockchain db is required, because of the “proof of context” extension ensuring the dynamic usage of memory in the mining loop, which helps 3DPass resist against FPGA devices sensitive to memory load speed.

Depending on the `pre_hash` calculated, there is going to be a directed sequence of `N` pseudo randomly chosen blocks to pick up (ex. `104786 → 476673 → 219964 → 891987 → 12897`) and prove its availability for the mining loop. Every mining loop will require another one different sequence to prove.

<img width="331" alt="proof_of_context" src="https://github.com/user-attachments/assets/44281dd0-e5ef-4c47-a160-dcc8bdaeceae" />

### Additional checks on 3D object shape consistency
- There is a rule on new block import, which requires: `Grid2d output 1 ≠ Grid2d output 2`. Therefore, the object shape must be complex enough to entirely avoid the Grid2d recognition error that is most likely to face with, especially, when it comes to scanning some regular- shaped objects (ex. a sphere). 

<img width="344" alt="collision_to_avoid" src="https://github.com/user-attachments/assets/b3df4e61-d4cd-483a-a2ba-08b6a3e47876" />

- In order to ensure 3D objects submitted as nonce meet the Grid2d recognition algorithm input requirements, there is a bunch of checks on the object shape being applied on the block import by both the Runtime and Native code, including but not limited to:
  - Vertex (..) points to an invalid halfedge
  - Halfedge (..) pointed to by vertex (..) does not start in that vertex, but instead in (..)
  - Vertex (..) does not point to a halfedge
  - Halfedge (..) points to an invalid twin halfedge (..)
  - Halfedge twin pointed to by halfedge (..) does not point back to halfedge
  - Invalid orientation: The halfedge (..) and its twin halfedge (..) points to the same vertex
  - Halfedge (..) does not point to a twin halfedge
  - Halfedge (..) points to an invalid vertex (..)
  - Halfedge (..) does not point to a vertex
  - Halfedge (..) points to an invalid face (..)
  - Halfedge (..) points to a face but not a next halfedge
  - Halfedge (..) points to an invalid next halfedge (..)
  - Halfedge (..) points to a next halfedge but not a face
  - Halfedge next pointed to by halfedge (..) does not point back to halfedge
  - Length of edge (..) is too small
  - Face (..) points to an invalid halfedge (..)
  - Halfedge pointed to by face (..) does not point to back to face
  - Face (..) does not point to a halfedge
  - Vertex (..) and Vertex (..) is connected one way, but not the other way
  - Vertex (..) and Vertex (..) is connected by multiple edges
  - Volume is lower than 0.1 * bound_volume (see the figure down below)

These checks are in place to ensure that 3D object’s surface is simply connected and has sufficient volume to impact on the center of inertia of mass.

## Why "1 Node = 1 Vote"?

The user object verification procedure, leveraged in the [3DPRC-2](https://github.com/3Dpass/whitepaper/blob/main/3DPRC-2.md) standard, is not considered trustworthy, unless the block production (both mining and block finalization aspects) are distributed enough to provide and safely deliver the asset copy protection service.

On top of the distribution aspect every Node must be responsible for its judgements on the user assets authenticity as much as for every block on the blockchain itself (judgements go into the block header). And, therefore, it must be equipped with a logically identical copy of the recognition toolkit [p3d](https://github.com/3Dpass/p3d) being leveraged as part of the [consensus protocol](https://3dpass.org/features#scanproof) in relation to both new block construction/verification and the user objects [HASH ID](https://3dpass.org/features#recognition-hash-id) creation/verification components.

<img width="641" alt="including_3DPRC-2_judgement" src="https://github.com/user-attachments/assets/96a4a7cb-c119-4478-add6-e33c6b635b95" />

Thus, neither of the Nodes has to share responsibility with less reliable entities connected to it like it happens in the classic mining pool situation (every Node must only construct blocks by itself thoughout the whole process). So, any classic mining pool running is limited by means of the Proof of Scan consensus logic.
