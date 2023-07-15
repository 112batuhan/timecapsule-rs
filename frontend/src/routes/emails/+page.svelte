<script lang="ts">
	import { goto } from '$app/navigation';
	import { PUBLIC_BACKEND_URL } from '$env/static/public';
	import { LoginState, loginStore } from '$lib/stores/loginState';
	import { Accordion, AccordionItem, Paginator } from '@skeletonlabs/skeleton';
	import { onMount } from 'svelte';
	import { modalStore } from '@skeletonlabs/skeleton';
	import type { ModalSettings } from '@skeletonlabs/skeleton';

	import DeleteSVG from './emailIcons/delete.svg?component';
	import EditSVG from './emailIcons/edit.svg?component';
	import EmailSendSVG from './emailIcons/emailSend.svg?component';
	import HideSVG from './emailIcons/hide.svg?component';
	import DuplicateSVG from './emailIcons/duplicate.svg?component';
	import { editorData, type EmailData } from '$lib/stores/editor';

	async function fetchMails() {
		try {
			const res = await fetch(PUBLIC_BACKEND_URL + '/email', {
				method: 'GET',
				credentials: 'include'
			});

			if (res.status == 200) {
				emails = await res.json();
				emails = emails.reverse();
				// Paginator email length update
				emails.forEach((item: any, i: number) => {
					item.display_id = i;
				});
				page.size = emails.length;
				active_item = 0;
			} else if (res.status == 204) {
				emails = [];
			} else if (res.status == 401) {
				$loginStore = LoginState.Not;
				goto('/');
			}
		} catch (error) {
			console.error(error);
		}
	}

	async function duplicateMail(email_id: string) {
		try {
			const res = await fetch(PUBLIC_BACKEND_URL + '/email/' + email_id + '/duplicate', {
				method: 'get',
				credentials: 'include'
			});

			if (res.status == 401) {
				$loginStore = LoginState.Not;
				goto('/');
			} else if (res.status == 403) {
				console.error("trying to duplicate a mail that doesn't belong to you!");
			} else {
				await fetchMails();
			}
		} catch (error) {
			console.error(error);
		}
	}

	async function deleteMail(email_id: string) {
		try {
			const res = await fetch(PUBLIC_BACKEND_URL + '/email/' + email_id, {
				method: 'DELETE',
				credentials: 'include'
			});

			if (res.status == 401) {
				$loginStore = LoginState.Not;
				goto('/');
			} else if (res.status == 403) {
				console.error("Trying to delete a mail that doesn't belong to you!");
			} else {
				await fetchMails();
			}
		} catch (error) {
			console.error(error);
		}
	}

	async function hideMail(email_id: string) {
		try {
			const res = await fetch(PUBLIC_BACKEND_URL + '/email/' + email_id + '/hide', {
				method: 'PATCH',
				credentials: 'include'
			});

			if (res.status == 401) {
				$loginStore = LoginState.Not;
				goto('/');
			} else if (res.status == 403) {
				console.error("Trying to hide a mail that doesn't belong to you!");
			} else {
				await fetchMails();
			}
		} catch (error) {
			console.error(error);
		}
	}

	onMount(async () => await fetchMails());

	let emails: any = [];

	let page = {
		offset: 0,
		limit: 5,
		size: emails.length,
		amounts: [1, 2, 5, 10]
	};
	$: paginatedEmails = emails.slice(
		page.offset * page.limit,
		page.offset * page.limit + page.limit
	);
	let active_item = -1;

	function calculateLastActivePage(amount_event: any) {
		if (active_item < 0) {
			page.offset = 0;
		} else {
			page.offset = Math.floor(active_item / amount_event.detail);
		}
	}

	function triggerDeleteModal(mailId: string) {
		new Promise<string>((resolve) => {
			const modal: ModalSettings = {
				type: 'confirm',

				title: 'Please confirm the deletion.',
				body: 'Are you sure you want to delete this e-mail? It will be deleted forever.',
				response: (r: boolean) => {
					if (r) {
						resolve(mailId);
					}
				}
			};
			modalStore.trigger(modal);
		}).then((mailId: string) => {
			deleteMail(mailId);
		});
	}

	function triggerHideModal(mailId: string) {
		new Promise<string>((resolve) => {
			const modal: ModalSettings = {
				type: 'confirm',

				title: 'Please confirm the hiding.',
				body: `Are you sure you want to hide this e-mail? 
						You won't be able to see, edit or duplicate this e-mail. 
						After hiding the e-mail, only deleting is allowed.`,
				response: (r: boolean) => {
					if (r) {
						resolve(mailId);
					}
				}
			};
			modalStore.trigger(modal);
		}).then((mailId: string) => {
			hideMail(mailId);
		});
	}

	function exportToEditor(mailData: EmailData) {
		$editorData = mailData;
		goto('/editor');
	}
</script>

<div class="card p-4 w-[100%] min-w-[600px]">
	{#if emails.length > 0}
		{#key page.size}
			<Paginator
				class="m-3"
				bind:settings={page}
				on:amount={calculateLastActivePage}
				showNumerals
				maxNumerals={1}
			/>
		{/key}
		<Accordion>
			{#each paginatedEmails as email}
				<AccordionItem
					on:toggle={() => (active_item = email.display_id)}
					class="variant-ghost-surface rounded-md"
					open={active_item == email.display_id}
				>
					<svelte:fragment slot="lead">&#9993;&#65039;</svelte:fragment>
					<svelte:fragment slot="summary">
						<div class="variant-ghost-surface p-1 grid rounded-2xl grid-cols-8 gap-4 items-center">
							<div class="pl-10 col-span-6"><strong>{email.subject}</strong></div>
							<div class="justify-self-center text-center">Send Date {email.send_date}</div>
							<div class="justify-self-center text-center">{email.is_html ? 'HTML' : 'TEXT'}</div>
						</div>
					</svelte:fragment>
					<svelte:fragment slot="content"
						><div class="justify-self-center flex flex-wrap justify-center">
							{#if !email.is_hidden || email.is_sent}
								<button
									class="btn variant-filled-primary p-1 m-1"
									on:click={async () => await duplicateMail(email.id)}
								>
									<DuplicateSVG class="w-5 h-5 mx-5" />
								</button>
							{/if}

							<button
								class="btn variant-filled-primary p-1 m-1"
								on:click={() => triggerDeleteModal(email.id)}
							>
								<DeleteSVG class="w-5 h-5 mx-5" />
							</button>
							{#if !email.is_hidden && !email.is_sent}
								<button
									class="btn variant-filled-primary p-1 m-1"
									on:click={() => {
										delete email.display_id;
										exportToEditor(email);
									}}
								>
									<EditSVG class="w-5 h-5 mx-5" />
								</button>
							{/if}
							{#if !email.is_hidden && !email.is_sent}
								<button
									class="btn variant-filled-primary p-1 m-1"
									on:click={() => triggerHideModal(email.id)}
								>
									<HideSVG class="w-5 h-5 mx-5" />
								</button>
							{/if}
							{#if !email.is_hidden && !email.is_sent}
								<button class="btn variant-filled-primary p-1 m-1">
									<EmailSendSVG class="w-7 h-7 mx-5" />
								</button>
							{/if}
						</div>
						{#if !email.is_hidden || email.is_sent}
							{#if email.is_sent}
								<div class="text-center">This e-mail has already been sent.</div>
							{/if}
							<div
								style="border-radius:0.3rem; background-color:#c2a6f5; 100%; height: 350px; overflow: hidden;"
							>
								{#if email.is_html}
									<iframe
										style="max-width: 100%; max-height: 100%; width: 100%; height: 100%;"
										title="preview"
										srcdoc={email.body}
									/>{:else}
									<iframe
										style="max-width: 100%; max-height: 100%; width: 100%; height: 100%; text-color:#000000;"
										title="preview"
										srcdoc={email.body}
									/>{/if}
							</div>
						{:else}
							<div class="text-center">This e-mail has been hidden.</div>
						{/if}
					</svelte:fragment>
				</AccordionItem>
			{/each}
		</Accordion>
	{:else}
		<p>No emails found.</p>
	{/if}
</div>
