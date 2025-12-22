<script>
    import { invoke } from '@tauri-apps/api/core';
    import { page } from '$app/state';

    import { getCurrentWindow } from '@tauri-apps/api/window';

    // State variables using Svelte 5 runes
    let serverAddress = $state('');
    let slideNumber = $state(0);
    let imageSrc = $state('');
    let showModal = $state(false);

    // Derived state for image source
    let imageUrl = $derived(
        slideNumber && serverAddress 
            ? `${serverAddress}/slide${slideNumber}.png` 
            : ''
    );

    $effect(() => {
        serverAddress = page.url.searchParams.get('serverAddress') || '';
        slideNumber = parseInt(page.url.searchParams.get('slideNumber') || '0');
        imageSrc = imageUrl;
    });

    async function handleDeleteConfirmed() {
        showModal = false;

        if (!slideNumber) {
            console.error("No slide number found for deletion.");
            return;
        }

        try {
            await invoke('delete_slide', { slideNumber });

            closeCurrentWindow();
            
        } catch (error) {
            console.error("Tauri command failed during slide deletion:", error);

            closeCurrentWindow();
        }
    }

    async function closeCurrentWindow() {
        try {
            await getCurrentWindow().close();
        } catch (error) {
            console.error("Failed to close the window:", error);
        }
    }

    function handleDeleteCanceled() {
        showModal = false;
    }

    function showDeleteConfirm() {
        showModal = true;
    }
</script>

<svelte:head>
    <title>Slide Viewer - {slideNumber}</title>
</svelte:head>

<div class="viewer-container">
    <img id="slide-image" src={imageSrc} alt={`Slide ${slideNumber}`} />

    <div id="controls">
        <button id="delete-button" onclick={showDeleteConfirm}>Delete Slide</button>
    </div>

    {#if showModal}
        <div id="custom-modal">
            <div id="modal-content">
                <p id="modal-message">Are you sure you want to delete this slide?</p>
                <div class="modal-buttons">
                    <button id="modal-ok" onclick={handleDeleteConfirmed}>OK</button>
                    <button id="modal-cancel" onclick={handleDeleteCanceled}>Cancel</button>
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    :global(html), :global(body) {
        margin: 0;
        padding: 0;
        height: 100%;
        width: 100%;
        overflow: hidden;
        background-color: #111;
    }

    .viewer-container {
        height: 100vh;
        width: 100vw;
    }

    #slide-image {
        display: block;
        width: 100%;
        height: 100%;
        object-fit: contain;
    }

    #controls {
        position: fixed;
        bottom: 20px;
        right: 20px;
        z-index: 1000;
    }

    #delete-button {
        padding: 10px 20px;
        font-size: 16px;
        background-color: #dc3545;
        color: white;
        border: none;
        border-radius: 5px;
        cursor: pointer;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
        transition: background-color 0.2s;
    }

    #delete-button:hover {
        background-color: #c82333;
    }
    
    #custom-modal {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0,0,0,0.7);
        z-index: 2000;
        display: flex; 
        justify-content: center;
        align-items: center;
    }

    #modal-content {
        background: white;
        padding: 25px;
        border-radius: 8px;
        box-shadow: 0 5px 15px rgba(0,0,0,0.4);
        width: 300px;
        text-align: center;
    }
    
    #modal-message {
        margin-bottom: 20px;
        font-size: 1.1em;
        color: #333;
    }

    .modal-buttons button {
        padding: 10px 20px;
        font-size: 15px;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        margin: 0 8px;
        transition: background-color 0.2s;
    }

    #modal-ok {
        background-color: #dc3545;
        color: white;
    }

    #modal-ok:hover {
        background-color: #c82333;
    }

    #modal-cancel {
        background-color: #f0f0f0;
        color: #333;
    }

    #modal-cancel:hover {
        background-color: #e0e0e0;
    }
</style>