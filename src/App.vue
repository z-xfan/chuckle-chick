<script setup lang="ts">
import PetStage from "@/components/PetStage.vue";
import SettingsView from "@/components/SettingsView.vue";

const isSettingsWindow = window.location.hash === "#settings";
</script>

<template>
  <main v-if="!isSettingsWindow" class="pet-window" aria-label="ChuckleChick 桌面宠物">
    <PetStage />
  </main>
  <SettingsView v-else />
</template>
