import WebSocket, { WebSocketServer } from 'ws';
import { MainContract } from "./target/Main.js";
import { AccountWallet, CompleteAddress, createLogger, Fr, PXE, waitForPXE, createPXEClient, Logger } from "@aztec/aztec.js";
import { getSchnorrAccount } from '@aztec/accounts/schnorr';
import { AztecAddress, deriveSigningKey } from '@aztec/circuits.js';
import { getInitialTestAccountsWallets } from "@aztec/accounts/testing";

const wss = new WebSocketServer({ port: 3001 });

const setupSandbox = async () => {
    const { PXE_URL = 'http://localhost:8080' } = process.env;
    const pxe = await createPXEClient(PXE_URL);
    await waitForPXE(pxe);
    return pxe;
};

async function deployContract() {
    console.log('goes into deployContract');
    let pxe: PXE = await setupSandbox();
    let wallets: AccountWallet[] = await getInitialTestAccountsWallets(pxe);
    let secretKey = Fr.random();
    let salt = Fr.random();
    let schnorrAccount = await getSchnorrAccount(pxe, secretKey, deriveSigningKey(secretKey), salt);
    let tx = await schnorrAccount.deploy().wait();
    let wallet = await schnorrAccount.getWallet();
    const mainContract = await MainContract.deploy(wallet).send().deployed();
    console.log(`✅ Main Contract deployed at: ${mainContract.address.toString()}`);
    return { contract: mainContract, wallet };
}

let contractInstance: any;
let walletInstance: any;

function startWebSocketServer(port = 3002) {
    try {
        const wss = new WebSocketServer({ port });

        wss.on('connection', (ws) => {
            console.log(`✅ Rust sequencer connected to WebSocket on port ${port}`);

            ws.on('message', async (message) => {
                console.log("📥 Received WebSocket message:", message.toString());
                ws.send(JSON.stringify({ success: true, message: "Test response from WebSocket server" }));
            });

            ws.on('close', () => {
                console.log(`❌ Rust sequencer disconnected from port ${port}`);
            });

        });

        console.log(`🚀 WebSocket server running on ws://localhost:${port}`);
    } catch (error: any) {
        if (error.code === 'EADDRINUSE') {
            console.error(`❌ Port ${port} already in use, retrying with port ${port + 1}...`);
            startWebSocketServer(port + 1); // Try next available port
        } else {
            console.error("❌ Unexpected error in WebSocket server:", error);
        }
    }
}

// // Start WebSocket server after contract deployment
// startWebSocketServer();

async function ensurePXEReady() {
    console.log("⏳ Waiting for PXE to be ready...");
    while (true) {
        try {
            const response = await fetch("http://localhost:8080/status");
            console.log('response.statusText', response.statusText);
            if (response.statusText === "OK") {
                console.log("✅ PXE is ready!");
                return;
            }
        } catch (error) {
            console.log("⏳ PXE not ready yet, retrying...");
        }
        await new Promise(resolve => setTimeout(resolve, 5000));
    }
}

async function deployContractWithRetry() {
    await ensurePXEReady();
    try {
        console.log("🚀 Deploying contract...");
        const { contract, wallet } = await deployContract();
        console.log("✅ Contract deployed at:", contract.address.toString());
        return { contract, wallet };
    } catch (error) {
        console.error("❌ Contract deployment failed, retrying in 10 seconds...", error);
        await new Promise(resolve => setTimeout(resolve, 10000));
        return deployContractWithRetry();
    }
}

deployContractWithRetry().then(({ contract, wallet }) => {
    contractInstance = contract;
    walletInstance = wallet;
    startWebSocketServer();
});
