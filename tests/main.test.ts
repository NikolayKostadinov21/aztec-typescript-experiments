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
    }, 100000);

    test('Sets new feeds', async () => {
        console.log('contract: ', contract.address);
        let x = getMainByAddress(AztecAddress.fromString("0x04bfd3ad859c1da7e45740d58ef55bd2195c20a63a383b460369f813ecfc1a24"), wallets[0]);
        let y = (await x).address;
        console.log("y: ", y);

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