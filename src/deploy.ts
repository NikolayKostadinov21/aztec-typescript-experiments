import { MainContract } from "./target/Main.js"
import { AccountWallet, CompleteAddress, createLogger, Fr, PXE, waitForPXE, createPXEClient, Logger } from "@aztec/aztec.js";
import { getSchnorrAccount } from '@aztec/accounts/schnorr';
import { AztecAddress, deriveSigningKey } from '@aztec/circuits.js';
import { getInitialTestAccountsWallets } from "@aztec/accounts/testing";
import { readFileSync } from "fs";

const setupSandbox = async () => {
    const { PXE_URL = 'http://localhost:8080' } = process.env;
    const pxe = await createPXEClient(PXE_URL);
    await waitForPXE(pxe);
    return pxe;
};

async function main() {
    console.log('here')
    let pxe: PXE;
    let wallets: AccountWallet[] = [];
    let accounts: CompleteAddress[] = [];
    let logger: Logger;

    logger = createLogger('aztec:aztec-typescript-experiments');
    console.log('after logger')

    pxe = await setupSandbox();
    console.log('pxe', pxe)
    wallets = await getInitialTestAccountsWallets(pxe);

    let secretKey = Fr.random();
    let salt = Fr.random();

    let schnorrAccount = await getSchnorrAccount(pxe, secretKey, deriveSigningKey(secretKey), salt);
    const { address, publicKeys, partialAddress } = await schnorrAccount.getCompleteAddress();
    let tx = await schnorrAccount.deploy().wait();
    let wallet = await schnorrAccount.getWallet();

    const mainContract = await MainContract.deploy(wallet).send().deployed();
    logger.info(`Main Contract deployed at: ${mainContract.address}`);

    const [owner] = await getInitialTestAccountsWallets(pxe);
    console.log('owner ----->', owner);
    const main = await getMain(mainContract.address, owner);
    console.log(main);

    console.log('before set_just_field');
    await main.methods.set_just_field(214).send().wait();
    const just_field = await main.methods.get_just_field().simulate();
    console.log('just_field: ', just_field);
}

export async function getMain(contractAddress: AztecAddress, wallet: any) {
    return MainContract.at(contractAddress, wallet);
}

main();
