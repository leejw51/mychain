import "mocha";
import chaiAsPromised = require("chai-as-promised");
import { use as chaiUse, expect } from "chai";
import { RpcClient } from "./core/rpc-client";
import { unbondAndWithdrawStake } from "./core/setup";
import {
	generateWalletName,
	newWalletRequest,
	newZeroFeeRpcClient,
	shouldTest,
	FEE_SCHEMA,
	newWithFeeRpcClient,
} from "./core/utils";
chaiUse(chaiAsPromised);

describe("Wallet management", () => {
	let client: RpcClient;
	before(async () => {
		await unbondAndWithdrawStake();
		if (shouldTest(FEE_SCHEMA.WITH_FEE)) {
			client = newWithFeeRpcClient();
		} else if (shouldTest(FEE_SCHEMA.ZERO_FEE)) {
			client = newZeroFeeRpcClient();
		}
	});

	it("can restore hd-wallet with specified name", async () => {
		const walletName = generateWalletName();
		const walletRequest = newWalletRequest(walletName, "123456");

		const walletRestoreResult = await client.request("wallet_restore", [
			walletRequest
			, "speed tortoise kiwi forward extend baby acoustic foil coach castle ship purchase unlock base hip erode tag keen present vibrant oyster cotton write fetch"
		]);
		expect(walletRestoreResult).to.deep.eq(walletName);

		const walletList = await client.request("wallet_list");
		expect(walletList).to.include(walletName);
	});



	it("cannot access un-existing wallet", async () => {
		const nonExistingWalletName = generateWalletName();
		const nonExistingWalletRequest = newWalletRequest(
			nonExistingWalletName,
			"123456",
		);

		await expect(
			client.request("wallet_listStakingAddresses", [nonExistingWalletRequest]),
		).to.eventually.rejectedWith(
			`Invalid input: Wallet with name (${nonExistingWalletName}) not found`,
		);
		await expect(
			client.request("wallet_listTransferAddresses", [nonExistingWalletRequest]),
		).to.eventually.rejectedWith(
			`Invalid input: Wallet with name (${nonExistingWalletName}) not found`,
		);
		await expect(
			client.request("wallet_balance", [nonExistingWalletRequest]),
		).to.eventually.rejectedWith(
			`Invalid input: Wallet with name (${nonExistingWalletName}) not found`,
		);
		await expect(
			client.request("wallet_transactions", [nonExistingWalletRequest]),
		).to.eventually.rejectedWith(
			`Invalid input: Wallet with name (${nonExistingWalletName}) not found`,
		);
	});

	it("can create wallet with specified name", async () => {
		const walletName = generateWalletName();
		const walletRequest = newWalletRequest(walletName, "123456");

		const walletCreateResult = await client.request("wallet_create", [
			walletRequest
			, "HD"
		]);
		let res= walletCreateResult.split(" ");
		expect(res.length).to.deep.eq(24);

		const walletList = await client.request("wallet_list");
		expect(walletList).to.include(walletName);
	});

	it("Newly created wallet has a staking and transfer address associated", async () => {
		const walletName = generateWalletName();
		const walletRequest = newWalletRequest(walletName, "123456");

		const walletCreateResponse = await client.request("wallet_create", [
			walletRequest, "HD"  
		]);
		let res= walletCreateResponse.split(" ");
		expect(res.length).to.deep.eq(24);


		const walletStakingAddresses = await client.request(
			"wallet_listStakingAddresses",
			[walletRequest],
		);
		expect(walletStakingAddresses).to.be.an("array");
		expect(walletStakingAddresses.length).to.eq(1);

		const walletTransferAddresses = await client.request(
			"wallet_listTransferAddresses",
			[walletRequest],
		);
		expect(walletTransferAddresses).to.be.an("array");
		expect(walletTransferAddresses.length).to.eq(1);
	});

	
	it("cannot create duplicated wallet", async () => {
		const walletName = generateWalletName();
		const walletRequest = newWalletRequest(walletName, "123456");

		const walletCreateResponse = await client.request("wallet_create", [
			walletRequest,"HD"
		]);
		let res= walletCreateResponse.split(" ");
		expect(res.length).to.deep.eq(24);

		return expect(
			client.request("wallet_create", [walletRequest,"HD"]),
		).to.eventually.rejectedWith(
			`HD Key with given name already exists`,
		);
	});

	it("User cannot access wallet with incorrect passphrase", async () => {
		const walletName = generateWalletName();
		const walletPassphrase = "passphrase";
		const walletRequest = newWalletRequest(walletName, walletPassphrase);

		const walletCreateResponse = await client.request("wallet_create", [
			walletRequest,"HD"
		]);
		let res= walletCreateResponse.split(" ");
		expect(res.length).to.deep.eq(24);



		const incorrectWalletPassphrase = "different_passphrase";
		const incorrectWalletRequest = newWalletRequest(
			walletName,
			incorrectWalletPassphrase,
		);

		await expect(
			client.request("wallet_listStakingAddresses", [incorrectWalletRequest]),
		).to.eventually.rejectedWith("Decryption error");
		await expect(
			client.request("wallet_listTransferAddresses", [incorrectWalletRequest]),
		).to.eventually.rejectedWith("Decryption error");
		await expect(
			client.request("wallet_balance", [incorrectWalletRequest]),
		).to.eventually.rejectedWith("Decryption error");
		await expect(
			client.request("wallet_transactions", [incorrectWalletRequest]),
		).to.eventually.rejectedWith("Decryption error");
	});

	it("Create a transfer address and then list it", async () => {
		const walletName = generateWalletName();
		const walletPassphrase = "passphrase";
		const walletRequest = newWalletRequest(walletName, walletPassphrase);

		
		const walletCreateResponse = await client.request("wallet_create", [
			walletRequest,"HD"
		]);
		let res= walletCreateResponse.split(" ");
		expect(res.length).to.deep.eq(24);

		const transferAddress = await client.request("wallet_createTransferAddress", [
			walletRequest,
		]);

		const transferAddressList = await client.request(
			"wallet_listTransferAddresses",
			[walletRequest],
		);
		expect(transferAddressList).to.be.an("array");
		expect(transferAddressList).to.include(transferAddress);
	});

	
	it("Create a staking address and then list it", async () => {
		const walletName = generateWalletName();
		const walletPassphrase = "passphrase";
		const walletRequest = newWalletRequest(walletName, walletPassphrase);

		
		const walletCreateResponse = await client.request("wallet_create", [
			walletRequest,"HD"
		]);
		let res= walletCreateResponse.split(" ");
		expect(res.length).to.deep.eq(24);

		const stakingAddress = await client.request("wallet_createStakingAddress", [
			walletRequest,
		]);

		const stakingAddressList = await client.request(
			"wallet_listStakingAddresses",
			[walletRequest],
		);
		expect(stakingAddressList).to.be.an("array");
		expect(stakingAddressList).to.include(stakingAddress);
	});
	
});
