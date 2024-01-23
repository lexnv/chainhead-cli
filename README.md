# chainhead-cli

1. Start a local substrate node

```bash
substrate-node --dev
```

2. Subscribe to the `chainHead_follow` events

```bash
cargo run subscribe

Subscription ID: Subscription(Str("Lhx0MrXt2d8ePoIN"))

ChainHead event: RawValue({"event":"initialized","finalizedBlockHash":"0xc6e3393ab9f351e3680129a24482aec045c06ec6ac14e94ca87df3376ffc3c3f"})

ChainHead event: RawValue({"blockHash":"0xae11fded61cfb968660ed7702eb8992253efd5b26262dc02f97d5cd98709ddfd","event":"newBlock","parentBlockHash":"0xc6e3393ab9f351e3680129a24482aec045c06ec6ac14e94ca87df3376ffc3c3f"})

ChainHead event: RawValue({"blockHash":"0xb7c877411a5f206c89b383647276049d4f296de72427750915fd955690f2b1eb","event":"newBlock","parentBlockHash":"0xae11fded61cfb968660ed7702eb8992253efd5b26262dc02f97d5cd98709ddfd"})

```

Make a note of the subscription ID and a block hash, you will need them for the storage call.


3. Make a storage call to retrieve data

```bash
cargo run storage Lhx0MrXt2d8ePoIN 0xb7c0ac7623527820413936f8b03fce6085b7ebbe165d83cd72ab868eec3e210b 0x3a636f6465

Storage response: RawValue({"discardedItems":0,"operationId":"0","result":"started"})
```

The first parameter is the subscription ID, the second one is the block hash, and the third parameter is the key to retrieve.
In this case, 0x3a636f6465 represents the well-known key ":code:".
