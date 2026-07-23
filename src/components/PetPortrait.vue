<script setup lang="ts">
withDefaults(defineProps<{ size?: number }>(), {
  size: 36,
});

const spritesheetUrl = "./pets/huangji-daxiao/spritesheet.png";
</script>

<template>
  <span
    class="pet-portrait"
    :style="{ width: `${size}px`, height: `${(size * 208) / 192}px` }"
    aria-hidden="true"
  >
    <img :src="spritesheetUrl" alt="" draggable="false" />
  </span>
</template>

<style scoped>
.pet-portrait {
  position: relative;
  display: inline-block;
  flex: 0 0 auto;
  overflow: hidden;
}

.pet-portrait img {
  position: absolute;
  top: 0;
  left: 0;
  width: 800%;
  height: 1100%;
  max-width: none;
  pointer-events: none;
}
</style>
