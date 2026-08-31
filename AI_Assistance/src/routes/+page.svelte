<script lang="ts">
  import BentoDashboard from "$lib/components/BentoDashboard.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";

  let aiResponseText = $state("");

  function handleStreamUpdate(value: string) {
    aiResponseText = value;
  }
</script>

<main class="min-h-screen w-full px-3 py-4 text-white sm:px-4 md:px-6 md:py-6 xl:px-8">
  <div class="mx-auto flex h-full w-full max-w-[1680px] flex-col gap-4 lg:gap-5">
    <SearchBar onStreamUpdate={handleStreamUpdate} />
    <BentoDashboard {aiResponseText} />
  </div>
</main>
