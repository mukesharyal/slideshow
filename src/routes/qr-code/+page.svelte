<script>
    import { page } from '$app/state';

    let serverAddress = $state('');

    $effect(() => {
        serverAddress = page.url.searchParams.get('serverAddress') || '';
    });

</script>


{#if serverAddress.length > 0}
    <div class='container'>
        <img src={`https://api.qrserver.com/v1/create-qr-code/?data=http://${serverAddress}&size=200x200&color=FF6600`} alt="QR Code for the address." />
        
        <h1>
            {serverAddress}
        </h1>
    </div>
{/if}



<style>
    .container{
        height: 100vh;
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        justify-content: center;
        gap: 5vh;
    }

    h1{
        font-size: 8vw;
        color: #ff6600;
    }
</style>
