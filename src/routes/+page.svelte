<script>

    import { invoke } from "@tauri-apps/api/core";

    import { listen } from "@tauri-apps/api/event";

    import { onMount } from "svelte";

    // This will be used to prevent multiple server starts
    let clicked = false;

    // The boolean to check whether the system is connected to a network or not
    let isConnected = $state(false);

    // The server address of the app, initially set to initial when the server hasn't been started
    let serverAddress = $state('initial');

    // The number of slides to synchronise the value between the frontend and backend of the app
    let numSlides = $state(0);

    // The variable to hold the recently removed slide
    // When this variable is changed, we will reload the images and if one of them belongs to this, we won't show that one
    let removedSlides = $state([]);

    // The variable to store which slide the user is currently at
    // Will be useful when the user presses forward and backward arrows a bunch of times and loses track
    // of whether every slide will be broadcasted or not
    let currentSlide = $state(1);

    // 1. Create a reactive variable for the list of slides to display
    let displayedSlides = $derived(Array.from({ length: numSlides }, (_, i) => i + 1)
        .filter(slideIndex => !removedSlides.includes(slideIndex)));


    (async () => {

        checkConnection();

    })();

    async function startServer()
    {
        if(clicked) return;


        invoke('start_server');

        clicked = true;
    }

    function openImage(slideNumber)
    {
        try
        {
            invoke('open_slide_viewer', {
                slideNumber: slideNumber,
                serverAddress: `http://${serverAddress}`
            });
        }
        catch(error)
        {
            alert(`The error is ${error}`);
        }
    }

    onMount(() => {

        listen('server_ready', (event) => {

            serverAddress = event.payload;
        });


        listen('new_slide', (event) => {

            numSlides = event.payload;
        });


        listen('slide_removed', (event) => {

            removedSlides.push(event.payload);
        });


        listen("volatile_slide_changed", (event) => {

            currentSlide = event.payload;
        });

    });

    async function checkConnection()
    {
        let connectionState = await invoke('is_connected');

        if(connectionState)
        {
            isConnected = true;
        }
    }


    async function showQR()
    {

        await invoke('show_qr_code', {
            serverAddress: serverAddress
        });

        
    }



</script>


<div class='container'>

    <div class={`header-container ${ numSlides > 0 ? 'shrunk' : '' }`}>

        <div class={`logo-container ${ numSlides > 0 ? 'apart' : 'centered' }`}>
            <img src='/logo.svg' alt="Slideshow Log" width='20%' />

            <h1 class='header'>
                Slideshow
            </h1>
        </div>

        {#if numSlides > 0}
            <button class='qr-button' onclick={showQR}>
                Show QR
            </button>
        {/if}

    </div>

        
    {#if numSlides === 0 }

        <div class='sub-header-container'>

            {#if isConnected}

                {#if serverAddress === 'initial'}

                    <button class='start-button' onclick={startServer}>
                        Start Server
                    </button>

                {:else}

                    <img src={`https://api.qrserver.com/v1/create-qr-code/?data=http://${serverAddress}&size=200x200&color=FF6600`} alt="QR Code for the address." />
                    
                    <h3 class='address'>
                        {serverAddress}
                    </h3>

                    <h3 class='instruction'>
                        Press <span class='highlight-key'>J</span> to capture the first slide
                    </h3>

                {/if}

            {:else}

                <img src='/internet.svg' alt="Slideshow Log" width='10%' />

                <h3 class='sub-header'>
                    Please connect to a network to begin
                </h3>

                <button class='check-button' onclick={checkConnection}>
                    Check Again
                </button>
            {/if}
        </div>

    {:else}

        <div class='slides-container'>

            {#each displayedSlides as originalIndex, i}

                <div class={`image-container ${ currentSlide === originalIndex ? 'selected' : '' }`}>
                    <img 
                        src={`http://${serverAddress}/slide${originalIndex}.png`} 
                        alt={`Slide ${originalIndex}`} 
                        onclick={() => {openImage(originalIndex)}} 
                    />

                    <p class='slide-number'>{i + 1}</p>
                </div>

            {/each}
            
        </div>

    {/if}

</div>


<style>

    .container{
        height: 100vh;
        display: flex;
        flex-direction: column;
        justify-content: space-between;
        padding-bottom: 10vh;
        box-sizing: border-box;
    }

    .header-container{
        display: flex;
        align-items: center;
        justify-content: center;
        background-color: #ffffff;
    }

    .header-container.shrunk{
        justify-content: space-between;
        padding-inline: 2.4vw;
        position: fixed;
        inset: 0 0 auto 0;
    }

    .header-container.shrunk .header{
        font-size: 4vw;
        line-height: 1;
    }

    .header-container.shrunk img{
        width: 4vw;
    }

    .header-container img{
        transform: rotate(90deg);
    }

    .logo-container{
        display: flex;
        align-items: center;
    }

    .logo-container.apart{
        justify-content: space-between;
    }

    .logo-container.centered{
        justify-content: center;
    }

    .qr-button{
        background-color: transparent;
        padding: 1vh;
        font-size: 2vw;
        color: #ff6600;
        font-weight: 600;
        border: none;
        border-radius: 1vh;
        cursor: pointer;
        transition: all 200ms ease-in-out;
        outline: 2px solid #ff6600;
    }

    .qr-button:hover{
        background-color: #ff6600;
        color: #ffffff;
    }

    .header{
        font-size: 10vw;
        color: #ff6600;
    }

    .sub-header{
        font-size: 4vw;
        margin-top: 0;
        color: #ff6600;
    }

    .instruction{
        font-size: 2vw;
        color: #ff6600;
    }

    .address{
        font-size: 4vw;
        color: #ff6600;
    }

    .sub-header-container{
        text-align: center;
    }

    .start-button, .check-button{
        background-color: #ff6600;
        width: 40vw;
        padding: 5vh;
        font-size: 4vw;
        font-weight: 600;
        color: #ffffff;
        border: none;
        border-radius: 2vh;
        cursor: pointer;
        transition: background-color 200ms ease-in-out;
    }

    .check-button{
        width: 30vw;
        padding: 2.5vh;
        border-radius: 1vh;
    }

    .start-button:hover, .check-button:hover{
        background-color: #E75C00;
    }

    .highlight-key{
        padding-block: 1vh;
        padding-inline: 2vh;
        background-color: #ff6600;
        color: #ffffff;
        border-radius: 1vh;
    }

    .slides-container{
        padding: 2.4vw;
        display: flex;
        align-items: flex-start;
        gap: 5vw;
        flex-wrap: wrap;
        padding-top: 11.2vw;
    }

    .slides-container img{
        width: 20vw;
        border-radius: 1vw;
        cursor: pointer;
    }

    .slide-number{
        text-align: center;
        color: #ff6600;
        font-weight: 600;
    }

    .image-container{
        margin: 0 auto;
        padding: 1rem;
        border-radius: 0.5vw;
    }

    .slides-container .selected{
        background-color: #FFD1B3;
    }

</style>




