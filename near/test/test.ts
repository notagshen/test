const nearAPI = require("near-api-js");
const BN = require("bn.js");
const fs = require("fs").promises;
const assert = require("assert").strict;
const fetch = require('node-fetch');
const elliptic = require("elliptic");
const web3Utils = require("web3-utils");

import {
    TestLib,
} from "./testlib";

import {
    ChainId,
    CHAIN_ID_ALGORAND,
    CHAIN_ID_NEAR,
} from "@certusone/wormhole-sdk/lib/cjs/utils";


function getConfig(env: any) {
  switch (env) {
    case "sandbox":
    case "local":
      return {
        networkId: "sandbox",
        nodeUrl: "http://localhost:3030",
        masterAccount: "test.near",
        wormholeAccount: Math.floor(Math.random() * 10000).toString() + "wormhole.test.near",
        tokenAccount: Math.floor(Math.random() * 10000).toString() + "token.test.near",
      };
  }
}

const wormholeMethods = {
  viewMethods: [],
  changeMethods: ["boot_wormhole", "submit_vaa"],
};
const tokenMethods = {
  viewMethods: [],
  changeMethods: ["boot_portal", "submit_vaa", "submit_vaa_callback"],
};

let config :any;
let masterAccount : any;
let masterKey : any;
let pubKey : any;
let keyStore : any;
let near : any;

async function initNear() {
  config = getConfig(process.env.NEAR_ENV || "sandbox");

  // Retrieve the validator key directly in the Tilt environment
  const response = await fetch('http://localhost:3031/validator_key.json');
  const keyFile = await response.json();

  masterKey = nearAPI.utils.KeyPair.fromString(
    keyFile.secret_key || keyFile.private_key
  );
  pubKey = masterKey.getPublicKey();
  keyStore = new nearAPI.keyStores.InMemoryKeyStore();
  keyStore.setKey(config.networkId, config.masterAccount, masterKey);
  near = await nearAPI.connect({
    deps: {
      keyStore,
    },
    networkId: config.networkId,
    nodeUrl: config.nodeUrl,
  });
  masterAccount = new nearAPI.Account(near.connection, config.masterAccount);
  console.log("Finish init NEAR: " + JSON.stringify(await masterAccount.getAccountBalance()));
}

async function createContractUser(
  accountPrefix : any,
  contractAccountId : any,
  methods : any
) {
  let accountId = Math.floor(Math.random() * 10000).toString() + accountPrefix + "." + config.masterAccount;

  console.log(accountId);

  let resp = await masterAccount.createAccount(
    accountId,
    pubKey,
    new BN(10).pow(new BN(25))
  );
  console.log("accountId: " + JSON.stringify(resp))
    
  keyStore.setKey(config.networkId, accountId, masterKey);
  const account = new nearAPI.Account(near.connection, accountId);
  const accountUseContract = new nearAPI.Contract(
    account,
    contractAccountId,
    methods
  );
  return accountUseContract;
}

async function initTest() {
  const wormholeContract = await fs.readFile("./contracts/wormhole/target/wasm32-unknown-unknown/release/wormhole.wasm");
  const tokenContract = await fs.readFile("./contracts/portal/target/wasm32-unknown-unknown/release/portal.wasm");

  const _wormholeAccount = await masterAccount.createAndDeployContract(
    config.wormholeAccount,
    pubKey,
    wormholeContract,
    new BN(10).pow(new BN(26))
  );

  const _tokenAccount = await masterAccount.createAndDeployContract(
    config.tokenAccount,
    pubKey,
    tokenContract,
    new BN(10).pow(new BN(26))
  );

  const wormholeUseContract = await createContractUser(
    "wormhole",
    config.wormholeAccount,
    wormholeMethods
  );

  const tokenUseContract = await createContractUser(
    "tokenbridge",
    config.tokenAccount,
    tokenMethods
  );

  console.log("Finish deploy contracts and create test accounts");
  return { wormholeUseContract, tokenUseContract };
}

async function test() {
    let fastTest = true;
    let ts = new TestLib();

    await initNear();
    const { wormholeUseContract, tokenUseContract } = await initTest();

    console.log("Booting guardian set with index 0");
    await wormholeUseContract.boot_wormhole({ args: { gset: 0, addresses: ts.guardianKeys} });
    console.log("Completed without an error... odd.. I am not sucking yet");

    console.log("Booting up the token bridge");
    await tokenUseContract.boot_portal({ args: { core: config.wormholeAccount } });
    console.log("token bridge booted");

    let seq = 1

    console.log("lets upgrade the governance set to 1");
    let vaa = ts.genGuardianSetUpgrade(ts.guardianPrivKeys, 0, 1, 1, seq, ts.guardianKeys);

    console.log("sending it to the core contract");
    await wormholeUseContract.submit_vaa({ args: { vaa : vaa }});

    seq = seq + 1

    if (!fastTest) {
        console.log("Its parsed... lets do it again!!");
        try {
            await wormholeUseContract.submit_vaa({ args: { vaa : vaa }});
            console.log("This should have thrown a exception..");
            process.exit(1);
        } catch { 
            console.log("Exception thrown.. nice.. we dont suck");
        }

        console.log("Lets try to send a governence message (SetFee) with the wrong index");
        vaa = ts.genGSetFee(ts.guardianPrivKeys, 0, 1, seq, CHAIN_ID_NEAR, 5);
        try {
            await wormholeUseContract.submit_vaa({ args: { vaa : vaa }});
            console.log("This should have thrown a exception..");
            process.exit(1);
        } catch { 
            console.log("Exception thrown.. nice..  this was with the wrong governance set");
        }

        console.log("Lets try to send a governence message (SetFee) with the correct index but the wrong chain");
        vaa = ts.genGSetFee(ts.guardianPrivKeys, 1, 1, seq, CHAIN_ID_ALGORAND, 5);
        try {
            await wormholeUseContract.submit_vaa({ args: { vaa : vaa }});
            console.log("This should have thrown a exception..");
            process.exit(1);
        } catch { 
            console.log("Exception thrown.. that is correct...   ");
        }

        console.log("Lets try to send a governence message (SetFee) with the correct index but for all chains");
        vaa = ts.genGSetFee(ts.guardianPrivKeys, 1, 1, seq, 0, 5);
        try {
            await wormholeUseContract.submit_vaa({ args: { vaa : vaa }});
            console.log("This should have thrown a exception..");
            process.exit(1);
        } catch { 
            console.log("Exception thrown.. that is correct...   ");
        }

        console.log("Lets try to send a governence message (SetFee)  with the correct index and the correct chain");

        vaa = ts.genGSetFee(ts.guardianPrivKeys, 1, 1, seq, CHAIN_ID_NEAR, 5);
        await wormholeUseContract.submit_vaa({ args: { vaa : vaa }});
        console.log("boo yaah! this was supposed to pass and it did");

        seq = seq + 1

        console.log("lets try to call the vaa_vallback directly");
        try {
            await tokenUseContract.submit_vaa_callback({ args: { }});
            console.log("This should have thrown a exception..");
            process.exit(1);

        } catch { 
            console.log("Exception thrown.. that is correct...   ");
        }

        try {
            vaa = ts.genRegisterChain(ts.guardianPrivKeys, 0, 1, seq, 1);
            console.log("Now lets call submit_vaa with a valid vaa (register the solana chain) on the token bridge.. with the wrong governance set");
            await tokenUseContract.submit_vaa({ args: { vaa: vaa }, gas: 300000000000000});
            console.log("This should have thrown a exception..");
            process.exit(1);
        } catch { 
            console.log("Exception thrown.. that is correct...   ");
        }
    }

    vaa = ts.genRegisterChain(ts.guardianPrivKeys, 1, 1, seq, 1);
    console.log("Now lets call submit_vaa with a valid vaa (register the solana chain) on the token bridge.. with the correct governance set");
    await tokenUseContract.submit_vaa({ args: { vaa: vaa }, gas: 300000000000000});

    seq = seq + 1

    try {
        vaa = ts.genRegisterChain(ts.guardianPrivKeys, 1, 1, seq, 1);
        console.log("Now lets call submit_vaa with a valid vaa (register the solana chain) again.. again... this should fail");
        await tokenUseContract.submit_vaa({ args: { vaa: vaa }, gas: 300000000000000});
        console.log("This should have thrown a exception..");
        process.exit(1);

    } catch { 
        console.log("Exception thrown.. that is correct...   ");
    }

    vaa = ts.genAssetMeta(ts.guardianPrivKeys, 1, 1, seq, "4523c3F29447d1f32AEa95BEBD00383c4640F1b4", 1, 8, "USDC", "CircleCoin");
    console.log("Now the fun stuff... lets create some USDC");
    await tokenUseContract.submit_vaa({ args: { vaa: vaa }, gas: 300000000000000});

    seq = seq + 1

    vaa = ts.genAssetMeta(ts.guardianPrivKeys, 1, 1, seq + 1, "4523c3F29447d1f32AEa95BEBD00383c4640F1b4", 1, 8, "USDC2", "CircleCoin2");
    console.log("Lets change the name and description");
    await tokenUseContract.submit_vaa({ args: { vaa: vaa }, gas: 300000000000000});

    try {
        vaa = ts.genAssetMeta(ts.guardianPrivKeys, 1, 1, seq, "4523c3F29447d1f32AEa95BEBD00383c4640F1b4", 1, 8, "USDC3", "CircleCoin3");
        console.log("Lets change the name and description.. using a older sequence number");
        await tokenUseContract.submit_vaa({ args: { vaa: vaa }, gas: 300000000000000});
        console.log("This should have thrown a exception..");
        process.exit(1);
    } catch { 
        console.log("Exception thrown.. that is correct...   ");
    }

    seq = seq + 2

    console.log("test complete");
}

test();
