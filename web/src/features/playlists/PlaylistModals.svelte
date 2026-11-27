<script lang="ts">
	import { fade, fly } from "svelte/transition";
	import { cubicIn, cubicOut } from "svelte/easing";
	import { Disc3, Image, ImageOff, Trash2, X } from "lucide-svelte";
	import { vynl } from "@lib/vynl";
	import { t } from "@lib/i18n";
	import { toasts } from "@lib/toast";
	import { blobSrc } from "@lib/format";
	import { focusTrap } from "@lib/actions/focus-trap";
	import { normalizeCoverImage } from "@lib/cover-image";
	import {
		createPlaylist,
		deletePlaylist,
		renamePlaylist,
		selectPlaylist,
		setPlaylistCover,
	} from "@state/playlists.svelte";
	import Dialog from "@components/Dialog.svelte";

	let {
		createOpen = $bindable(false),
		editTarget = $bindable(null),
		onnavigate,
	}: {
		createOpen: boolean;
		editTarget: { id: string; name: string; cover: string | null } | null;
		onnavigate: (page: "playlist") => void;
	} = $props();

	let createBusy = $state(false);
	let createVal = $state("");
	let createCoverFile = $state<File | null>(null);
	let createCoverPreview = $state<string | null>(null);
	let createCoverInput: HTMLInputElement | undefined = $state();

	let editRename = $state("");
	let editBusy = $state(false);
	let editDeleteConfirm = $state(false);
	let editCoverInput: HTMLInputElement | undefined = $state();

	$effect(() => {
		if (createOpen) {
			createVal = "";
			removeCreateCover();
		}
	});

	$effect(() => {
		if (editTarget) {
			editRename = editTarget.name;
			editDeleteConfirm = false;
		}
	});

	function pickCreateCover(): void {
		createCoverInput?.click();
	}

	async function onCreateCoverFile(e: Event): Promise<void> {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		input.value = "";
		if (!file) return;
		if (createCoverPreview) URL.revokeObjectURL(createCoverPreview);
		createCoverFile = file;
		createCoverPreview = URL.createObjectURL(file);
	}

	function removeCreateCover(): void {
		if (createCoverPreview) URL.revokeObjectURL(createCoverPreview);
		createCoverFile = null;
		createCoverPreview = null;
	}

	async function confirmCreatePlaylist(): Promise<void> {
		if (createBusy || !createVal.trim()) return;
		createBusy = true;
		const name = createVal.trim();
		const coverFile = createCoverFile;
		const coverPreview = createCoverPreview;
		createOpen = false;
		try {
			const pl = await createPlaylist(name);
			if (coverFile) {
				try {
					const { ext, data } = await normalizeCoverImage(coverFile);
					const savedPath = await vynl.savePlaylistCover(
						pl.id,
						ext,
						data,
					);
					await setPlaylistCover(pl.id, savedPath);
				} catch (err) {
					console.warn("set playlist cover failed:", err);
					toasts.error(t("playlist.coverFailed"));
				} finally {
					if (coverPreview) URL.revokeObjectURL(coverPreview);
				}
			}
			await selectPlaylist(pl.id);
			onnavigate("playlist");
		} catch (err) {
			console.warn("create playlist failed:", err);
			toasts.error(t("playlists.createFailed"));
		} finally {
			createBusy = false;
			createCoverFile = null;
			createCoverPreview = null;
		}
	}

	function pickEditCover(): void {
		editCoverInput?.click();
	}

	async function onEditCoverFile(e: Event): Promise<void> {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		input.value = "";
		if (!file || !editTarget || editBusy) return;
		const target = editTarget;
		editBusy = true;
		try {
			const { ext, data } = await normalizeCoverImage(file);
			const savedPath = await vynl.savePlaylistCover(
				target.id,
				ext,
				data,
			);
			await setPlaylistCover(target.id, savedPath);
			if (editTarget?.id === target.id) {
				editTarget = { ...editTarget, cover: savedPath };
			}
		} catch (err) {
			console.warn("set playlist cover failed:", err);
			toasts.error(t("playlist.coverFailed"));
		} finally {
			editBusy = false;
		}
	}

	async function saveEditRename(): Promise<void> {
		if (!editTarget || editBusy) return;
		const next = editRename.trim();
		if (!next || next === editTarget.name) {
			editTarget = null;
			return;
		}
		editBusy = true;
		try {
			await renamePlaylist(editTarget.id, next);
			editTarget = null;
		} catch (err) {
			console.warn("rename playlist failed:", err);
			toasts.error(t("playlist.renameFailed"));
		} finally {
			editBusy = false;
		}
	}

	async function removeEditCover(): Promise<void> {
		if (!editTarget || editBusy) return;
		editBusy = true;
		try {
			await setPlaylistCover(editTarget.id, null);
			editTarget = { ...editTarget, cover: null };
		} catch (err) {
			console.warn("remove playlist cover failed:", err);
			toasts.error(t("playlist.coverFailed"));
		} finally {
			editBusy = false;
		}
	}

	async function confirmEditDeletePlaylist(): Promise<void> {
		if (!editTarget || editBusy) return;
		editBusy = true;
		try {
			await deletePlaylist(editTarget.id);
			editDeleteConfirm = false;
			editTarget = null;
		} catch (err) {
			console.warn("delete playlist failed:", err);
			toasts.error(t("playlists.deleteFailed"));
		} finally {
			editBusy = false;
		}
	}
</script>

{#if createOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
	<div
		class="backdrop"
		role="presentation"
		transition:fade={{ duration: 150 }}
		onclick={() => {
			if (!createBusy) createOpen = false;
		}}
	>
		<div
			class="edit-panel"
			role="dialog"
			aria-modal="true"
			aria-label={t("common.newPlaylistTitle")}
			tabindex="-1"
			in:fly={{ y: 8, duration: 180, easing: cubicOut }}
			out:fly={{ y: 8, duration: 140, easing: cubicIn }}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === "Escape" && !e.defaultPrevented && !createBusy) {
					e.preventDefault();
					createOpen = false;
				}
			}}
			use:focusTrap
		>
			<div class="edit-head">
				<span class="edit-title">{t("common.newPlaylistTitle")}</span>
				<button
					class="edit-close"
					onclick={() => (createOpen = false)}
					disabled={createBusy}
					aria-label={t("common.close")}
				>
					<X size={12} stroke-width={1.6} />
				</button>
			</div>
			<div class="edit-hero">
				<div class="edit-cover">
					{#if createCoverPreview}
						<img
							class="edit-cover-img"
							src={createCoverPreview}
							alt=""
							draggable="false"
						/>
					{:else}
						<div class="edit-cover-placeholder">
							<Disc3 size={24} stroke-width={1} />
						</div>
					{/if}
					<div class="edit-cover-overlay">
						<button
							class="cover-action"
							onclick={pickCreateCover}
							aria-label={t("playlist.changeCover")}
						>
							<Image size={14} stroke-width={1.5} />
						</button>
						{#if createCoverPreview}
							<button
								class="cover-action"
								onclick={removeCreateCover}
								aria-label={t("playlist.removeCover")}
							>
								<ImageOff size={14} stroke-width={1.5} />
							</button>
						{/if}
					</div>
				</div>
				<div class="edit-field">
					<label class="edit-label" for="pl-create-name">
						{t("playlists.newDesc")}
					</label>
					<div class="edit-row">
						<input
							id="pl-create-name"
							class="dlg-input"
							type="text"
							bind:value={createVal}
							spellcheck={false}
							onkeydown={(e) => {
								if (e.key === "Enter")
									void confirmCreatePlaylist();
							}}
						/>
						<button
							class="action-btn-sm"
							onclick={() => void confirmCreatePlaylist()}
							disabled={createBusy || !createVal.trim()}
						>
							{t("common.create")}
						</button>
					</div>
				</div>
			</div>
			<input
				bind:this={createCoverInput}
				type="file"
				accept="image/*"
				class="cover-file-input"
				onchange={onCreateCoverFile}
			/>
		</div>
	</div>
{/if}

{#if editTarget}
	<div
		class="backdrop"
		role="presentation"
		transition:fade={{ duration: 150 }}
		onclick={() => {
			if (!editBusy) editTarget = null;
		}}
	>
		<div
			class="edit-panel"
			role="dialog"
			aria-modal="true"
			aria-label={t("playlist.editPlaylist")}
			tabindex="-1"
			in:fly={{ y: 8, duration: 180, easing: cubicOut }}
			out:fly={{ y: 8, duration: 140, easing: cubicIn }}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === "Escape" && !editBusy) {
					e.preventDefault();
					editTarget = null;
				}
			}}
			use:focusTrap
		>
			<div class="edit-head">
				<span class="edit-title">{t("playlist.editPlaylist")}</span>
				<button
					class="edit-close"
					onclick={() => (editTarget = null)}
					disabled={editBusy}
					aria-label={t("common.cancel")}
				>
					<X size={12} stroke-width={1.6} />
				</button>
			</div>
			<div class="edit-hero">
				<div class="edit-cover">
					{#if editTarget.cover}
						<img
							class="edit-cover-img"
							use:blobSrc={editTarget.cover}
							alt=""
							loading="lazy"
							draggable="false"
						/>
					{:else}
						<div class="edit-cover-placeholder">
							<Disc3 size={24} stroke-width={1} />
						</div>
					{/if}
					<div class="edit-cover-overlay">
						<button
							class="cover-action"
							onclick={pickEditCover}
							disabled={editBusy}
							aria-label={t("playlist.changeCover")}
						>
							<Image size={14} stroke-width={1.5} />
						</button>
						{#if editTarget.cover}
							<button
								class="cover-action"
								onclick={() => void removeEditCover()}
								disabled={editBusy}
								aria-label={t("playlist.removeCover")}
							>
								<ImageOff size={14} stroke-width={1.5} />
							</button>
						{/if}
					</div>
				</div>
				<div class="edit-field">
					<label class="edit-label" for="pl-rename-side">
						{t("playlists.renameTitle")}
					</label>
					<div class="edit-row">
						<input
							id="pl-rename-side"
							class="dlg-input"
							type="text"
							bind:value={editRename}
							spellcheck={false}
							onkeydown={(e) => {
								if (e.key === "Enter") void saveEditRename();
							}}
						/>
						<button
							class="action-btn-sm"
							onclick={() => void saveEditRename()}
							disabled={editBusy || !editRename.trim()}
						>
							{t("common.save")}
						</button>
					</div>
					<button
						class="edit-action"
						onclick={() => (editDeleteConfirm = true)}
						disabled={editBusy}
					>
						<Trash2 size={14} stroke-width={1.5} />
						<span>{t("playlists.deleteTitle")}</span>
					</button>
				</div>
			</div>
			<input
				bind:this={editCoverInput}
				type="file"
				accept="image/*"
				class="cover-file-input"
				onchange={onEditCoverFile}
			/>
		</div>
	</div>
{/if}

<Dialog
	open={editDeleteConfirm && !!editTarget}
	title={t("playlists.deleteTitle")}
	description={editTarget
		? t("playlists.deleteBody", { name: editTarget.name })
		: ""}
	confirmLabel={t("common.delete")}
	danger
	confirmDisabled={editBusy}
	onConfirm={() => void confirmEditDeletePlaylist()}
	onClose={() => {
		if (!editBusy) editDeleteConfirm = false;
	}}
/>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: var(--z-modal);
		padding: 16px;
		background: var(--overlay-bg);
		backdrop-filter: blur(var(--overlay-blur));
	}
	.edit-panel {
		background: var(--surface);
		border: 1px solid var(--line-strong);
		border-radius: var(--radius-sm);
		padding: 16px;
		width: min(420px, 100%);
		display: flex;
		flex-direction: column;
		gap: 10px;
		box-shadow:
			0 24px 64px rgba(0, 0, 0, 0.55),
			inset 0 1px 0 rgba(255, 255, 255, 0.03);
	}
	.edit-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}
	.edit-title {
		font-size: 15px;
		font-weight: 600;
		color: var(--text);
		letter-spacing: -0.01em;
	}
	.edit-close {
		width: 24px;
		height: 24px;
		border-radius: var(--radius-sm);
		background: var(--bg-raise);
		border: 1px solid var(--line);
		color: var(--faint);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transform: rotate(45deg);
	}
	.edit-close:hover {
		color: var(--text);
		border-color: var(--line-strong);
	}
	.edit-close :global(svg) {
		transform: rotate(-45deg);
	}
	.edit-hero {
		display: flex;
		gap: 12px;
		align-items: center;
	}
	.edit-cover {
		position: relative;
		width: 112px;
		height: 112px;
		flex: 0 0 112px;
		border-radius: var(--radius-sm);
		overflow: hidden;
		background: var(--bg-raise);
		border: 1px solid var(--line);
	}
	.edit-cover-img,
	.edit-cover-placeholder {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.edit-cover-placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--faint);
	}
	.edit-cover-overlay {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px;
		background: rgba(0, 0, 0, 0.42);
		opacity: 0;
		transition: opacity 0.14s ease;
	}
	.edit-cover:hover .edit-cover-overlay,
	.edit-cover:focus-within .edit-cover-overlay {
		opacity: 1;
	}
	@media (hover: none) {
		.edit-cover-overlay {
			opacity: 1;
		}
	}
	.cover-action {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 30px;
		height: 30px;
		border: 1px solid rgba(255, 255, 255, 0.14);
		border-radius: var(--radius-sm);
		background: rgba(18, 18, 18, 0.5);
		color: #fff;
		cursor: pointer;
		backdrop-filter: blur(6px);
	}
	.cover-action:hover:not(:disabled) {
		background: rgba(18, 18, 18, 0.72);
	}
	.cover-action:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.edit-hero .edit-field {
		flex: 1;
		min-width: 0;
	}
	.edit-field {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.edit-label {
		font-size: 11px;
		color: var(--dim);
		letter-spacing: 0.05em;
		text-transform: uppercase;
	}
	.edit-row {
		display: flex;
		gap: 8px;
		align-items: center;
	}
	.edit-row .dlg-input {
		flex: 1;
		min-width: 0;
	}
	.action-btn-sm {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 8px 12px;
		border-radius: var(--radius-sm);
		border: 1px solid var(--line-strong);
		background: var(--bg-raise);
		color: var(--text);
		font: 500 12px inherit;
		cursor: pointer;
		white-space: nowrap;
	}
	.action-btn-sm:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.edit-action {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		width: 100%;
		padding: 8px 12px;
		border-radius: var(--radius-sm);
		border: 1px solid var(--line);
		background: transparent;
		color: var(--text);
		font: 500 12px inherit;
		cursor: pointer;
	}
	.edit-action:hover:not(:disabled) {
		background: var(--bg-raise);
	}
	.edit-action:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.cover-file-input {
		display: none;
	}
</style>
