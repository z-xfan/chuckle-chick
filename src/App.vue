<script setup lang="ts">
import PetStage from "@/components/PetStage.vue";
import SettingsView from "@/components/SettingsView.vue";
import AssistantView from "@/components/AssistantView.vue";

const isSettingsWindow = window.location.hash === "#settings";
const isAssistantWindow = window.location.hash === "#assistant";
</script>

<template>
  <AssistantView v-if="isAssistantWindow" />
  <SettingsView v-else-if="isSettingsWindow" />
  <main v-else class="pet-window" aria-label="ChuckleChick 桌面宠物">
    <PetStage />
  </main>
</template>
