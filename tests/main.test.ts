import { AccountWallet, CompleteAddress, ContractDeployer, Fr, PXE, waitForPXE, TxStatus, createPXEClient, getContractInstanceFromDeployParams, Contract, AztecAddress, } from "@aztec/aztec.js";
import { getInitialTestAccountsWallets } from "@aztec/accounts/testing"
import {
    MainContract, MainContractArtifact
} from '../src/target/Main.js';
import { beforeAll, beforeEach, describe, expect, test } from 'vitest';

export const setupSandbox = async () => {
    const { PXE_URL = 'http://localhost:8080' } = process.env;
    const pxe = createPXEClient(PXE_URL);
    await waitForPXE(pxe);
    return pxe;
};

function getMainByAddress(contractAddress: AztecAddress, wallet: any) {
    return MainContract.at(contractAddress, wallet);
}

let pxe: PXE;
let wallets: AccountWallet[] = [];
let accounts: CompleteAddress[] = [];
let contract: Contract;

describe('Reading from/Writing to storage', () => {
    beforeEach(async () => {
        pxe = await setupSandbox();
        wallets = await getInitialTestAccountsWallets(pxe);
        accounts = wallets.map(w => w.getCompleteAddress());
        contract = await MainContract.deploy(wallets[0])
            .send()
            .deployed();

        console.log('new contract: ', contract.address);
    }, 100000);

    test('Sets new feeds', async () => {
        console.log('contract address inside the test: ', contract.address);
        let x = (await getMainByAddress(contract.address, wallets[0]));
        console.log("fn selector of set_ju8st_field is ", (await getMainByAddress(contract.address, wallets[0])).methods.set_just_field.selector)
        let y = (x).address;
        console.log("inside the test > address of the instance of the contract: ", y);

        await contract.methods.set_just_field(1).send().wait();

        // await contract
        //     .withWallet(wallets[0])
        //     .methods.set_just_field(1)
        //     .send()
        //     .wait();

        // let x = await contract
        //     .withWallet(wallets[0])
        //     .methods.get_just_field()
        //     .simulate();
        // console.log('result x is: ', x);
    });
});