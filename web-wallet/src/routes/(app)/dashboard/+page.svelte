<svelte:options immutable={true}/>

<script>
	import { onDestroy } from "svelte";
	import { fade } from "svelte/transition";
	import {
		filterWith,
		find,
		hasKeyValue,
		last
	} from "lamb";
	import {
		mdiContain,
		mdiDatabaseOutline,
		mdiSwapVertical
	} from "@mdi/js";

	import {
		Icon,
		Tabs
	} from "$lib/dusk/components";
	import {
		StakeContract,
		TransferContract
	} from "$lib/containers";
	import {
		AddressPicker,
		Balance,
		Transactions
	} from "$lib/components";
	import {
		operationsStore,
		settingsStore,
		walletStore
	} from "$lib/stores";
	import { contractDescriptors } from "$lib/contracts";

	/** @type {import('./$types').PageData} */
	export let data;

	const { currentPrice } = data;

	const {
		currency,
		dashboardTransactionLimit,
		language
	} = $settingsStore;

	/** @type {(descriptors: ContractDescriptor[]) => ContractDescriptor[]} */
	const getEnabledContracts = filterWith(hasKeyValue("disabled", false));

	/** @param {string} id */
	function updateOperation (id) {
		operationsStore.update((store) => ({
			...store,
			currentOperation: id
		}));
	}

	/**
	 * @param {keyof import("$lib/stores/stores").SettingsStore} property
	 * @param {any} value
	 */
	function updateSetting (property, value) {
		settingsStore.update((store) => ({
			...store,
			[property]: value
		}));
	}

	const enabledContracts = getEnabledContracts(contractDescriptors);
	const hasNoEnabledContracts = enabledContracts.length === 0;

	const tabItems = enabledContracts.map(({ id, label }) => ({
		icon: { path: id === "transfer" ? mdiSwapVertical : mdiDatabaseOutline },
		id,
		label
	}));

	let selectedTab = tabItems[0]?.id ?? "";

	$: selectedContract = find(enabledContracts, hasKeyValue("id", selectedTab));
	$: ({ balance, currentAddress, addresses } = $walletStore);
	$: ({ currentOperation } = $operationsStore);

	onDestroy(() => {
		updateOperation("");
	});
</script>

<div class="dashboard-content">
	<h2 class="visible-hidden">Dashboard</h2>

	<AddressPicker
		{addresses}
		{currentAddress}
	/>

	<Balance
		fiatCurrency={currency}
		fiatPrice={currentPrice[currency.toLowerCase()]}
		locale={language}
		tokenCurrency="DUSK"
		tokens={balance.value}
	/>

	{#if hasNoEnabledContracts}
		<div class="no-contracts-pane">
			<Icon path={mdiContain} size="large"/>
			<h3>No Contracts Enabled</h3>
			<p>It appears that no contracts are currently enabled.
				To access the full range of functionalities, enabling contracts is essential.</p>
			<h4>For Developers:</h4>
			<p>If you're in the midst of development and have encountered this message,
				it's possible that the necessary contract settings
				have not been configured or activated as expected.</p>
			<h4>Need Assistance?</h4>
			<p>Our support team is ready to assist with any questions or
				challenges you may encounter regarding contract configuration and activation.</p>
		</div>
	{/if}

	{#if selectedContract}
		<article class="tabs">
			<Tabs
				bind:selectedTab
				items={tabItems}
				on:change={() => updateOperation("")}
			/>
			<div
				class="tabs__panel"
				class:tabs__panel--first={selectedTab === enabledContracts[0].id}
				class:tabs__panel--last={selectedTab === last(enabledContracts).id}
			>
				{#key selectedTab}
					<div in:fade class="tabs__contract">
						<svelte:component
							descriptor={selectedContract}
							on:suppressStakingNotice={() => updateSetting("hideStakingNotice", true)}
							on:operationChange={({ detail }) => updateOperation(detail)}
							this={selectedTab === "transfer" ? TransferContract : StakeContract}
						/>
					</div>
				{/key}
			</div>
		</article>
	{/if}

	{#if currentOperation === "" && selectedTab === "transfer" }
		<Transactions
			items={walletStore.getTransactionsHistory()}
			{language}
			limit={dashboardTransactionLimit}/>
	{/if}
</div>

<style lang="postcss">
	.dashboard-content {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 1.375rem;
		overflow-y: auto;
		flex: 1;
	}

	.tabs {
		&__panel {
			border-radius: var(--control-border-radius-size);
			background: var(--surface-color);
			transition: border-radius 0.4s ease-in-out;

			&--first {
				border-top-left-radius: 0;
			}

			&--last {
				border-top-right-radius: 0;
			}
		}

		&__contract {
			display: flex;
			flex-direction: column;
			padding: 1rem 1.375rem;
			gap: var(--default-gap);
		}
	}

	.no-contracts-pane {
		display: flex;
		flex-direction: column;
		background-color: var(--surface-color);
		padding: 1rem 1.375rem;
		border-radius: var(--control-border-radius-size);

		& h3 {
			text-align: center;
			margin-bottom: 1em;
		}

		& p:not(:last-child) {
			margin-bottom: 1em;
		}

		h4 {
			margin-bottom: .5em;
		}

		:global(.dusk-icon) {
			align-self: center;
			margin-bottom: .5rem;
		}
	}
</style>
