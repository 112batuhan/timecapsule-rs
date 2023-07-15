<script lang="ts">
	import { PUBLIC_BACKEND_URL } from '$env/static/public';
	import HtmlEditor from '$lib/components/HtmlEditor.svelte';
	import TextEditor from '$lib/components/TextEditor/TextEditor.svelte';
	import BeforeUnload from '$lib/components/beforeUnload.svelte';
	import { editorData } from '$lib/stores/editor';
	import { inputState, type Input, checkEmpty, setInputStyling } from '$lib/types/inputFields';
	import SaveSVG from './save.svg?component';
	import { LoginState, loginStore } from '$lib/stores/loginState';
	import { goto } from '$app/navigation';

	$: subjectInput = { state: inputState.Idle, value: $editorData.subject } as Input;
	$: dateInput = { state: inputState.Idle, value: $editorData.send_date } as Input;

	$: if (Date.parse($editorData.send_date) <= Date.now()) {
		$editorData.send_date = new Date().toISOString().split('T')[0];
	}

	function checkInputs(): boolean {
		console.log($editorData);
		let subjectResultTuple = checkEmpty(subjectInput);
		subjectInput = subjectResultTuple[0];
		let dateResultTuple = checkEmpty(dateInput);
		dateInput = dateResultTuple[0];
		if (subjectResultTuple[1] || dateResultTuple[1]) {
			console.log(subjectInput);
			return false;
		}
		return true;
	}

	async function update_email() {
		if (!checkInputs()) {
			return;
		}
		try {
			const res = await fetch(PUBLIC_BACKEND_URL + '/email/' + $editorData.id, {
				method: 'PATCH',
				credentials: 'include',
				body: JSON.stringify({
					is_html: $editorData.is_html,
					subject: $editorData.subject,
					date: $editorData.send_date,
					body: $editorData.body
				}),
				headers: {
					'Content-Type': 'application/json'
				}
			});

			if (res.status == 200) {
				goto('/emails');
			} else if (res.status == 403) {
				console.error("Trying to delete a mail that doesn't belong to you!");
			} else if (res.status == 401) {
				$loginStore = LoginState.Not;
				goto('/');
			}
		} catch (error) {
			console.error(error);
		}
	}
</script>

<BeforeUnload />

<div class="card p-4 w-[100%] min-w-[600px]">
	<div class="flex justify-center gap-4 w-full my-2 items-end">
		<label class="w-3/4">
			<span class="ml-3"> Subject </span>
			<input
				type="text"
				bind:value={$editorData.subject}
				on:focus={() => (subjectInput.state = inputState.Idle)}
				class="input my-2 {setInputStyling(subjectInput.state)}"
			/>
		</label>
		<label class="w-1/4">
			<span class="ml-3"> Send Date </span>
			<input
				type="date"
				bind:value={$editorData.send_date}
				on:focus={() => (dateInput.state = inputState.Idle)}
				class="input my-2 {setInputStyling(dateInput.state)}"
			/>
		</label>
		<button class="btn variant-ghost-primary my-2" on:click={async () => await update_email()}>
			<SaveSVG class="w-6 h-6" />
		</button>
	</div>

	{#if $editorData.is_html}
		<HtmlEditor bind:text={$editorData.body} />
	{:else}
		<TextEditor initialContent={$editorData.body} bind:exportContent={$editorData.body} />
	{/if}
</div>
