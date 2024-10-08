<template>
  <div
    v-if="component"
    :class="
      clsx(
        'p-xs border-l-4 border relative',
        titleCard ? 'mb-xs' : 'rounded-md',
        component.toDelete && 'opacity-70',
        component.fromBaseChangeSet && 'opacity-70',
      )
    "
    :style="{
      borderColor: component.color,
      backgroundColor: `#${bodyBg.toHex()}`,
    }"
  >
    <div class="flex gap-2xs items-center">
      <Icon :name="component.icon" size="lg" class="shrink-0" />
      <Icon
        :name="COMPONENT_TYPE_ICONS[component.componentType]"
        size="lg"
        class="shrink-0"
      />
      <Stack spacing="xs" class="">
        <div
          ref="componentNameRef"
          v-tooltip="componentNameTooltip"
          class="font-bold break-all line-clamp-4 pb-[1px]"
        >
          {{ component.displayName }}
        </div>
        <div class="text-xs italic capsize">
          <div class="truncate pr-xs">{{ component.schemaName }}</div>
        </div>
      </Stack>

      <!-- ICONS AFTER THIS POINT ARE RIGHT ALIGNED DUE TO THE ml-auto STYLE ON THIS DIV -->
      <div
        v-tooltip="{
          content: 'Upgrade',
          theme: 'instant-show',
        }"
        class="ml-auto cursor-pointer rounded hover:scale-125"
      >
        <StatusIndicatorIcon
          v-if="component.canBeUpgraded"
          type="upgradable"
          @click="upgradeComponent"
        />
      </div>

      <!-- change status icon -->
      <div
        v-if="component.changeStatus !== 'unmodified'"
        v-tooltip="{
          content:
            component.changeStatus.charAt(0).toUpperCase() +
            component.changeStatus.slice(1),
          theme: 'instant-show',
        }"
        class="cursor-pointer rounded hover:scale-125"
      >
        <StatusIndicatorIcon
          type="change"
          :status="component.changeStatus"
          @click="componentsStore.setComponentDetailsTab('diff')"
        />
      </div>

      <!-- Slot for additional icons/buttons -->
      <slot />
    </div>
  </div>
</template>

<script lang="ts" setup>
import { computed, PropType, ref } from "vue";
import tinycolor from "tinycolor2";
import clsx from "clsx";
import {
  useTheme,
  Icon,
  Stack,
  COMPONENT_TYPE_ICONS,
} from "@si/vue-lib/design-system";
import { FullComponent, useComponentsStore } from "@/store/components.store";
import { ComponentId } from "@/api/sdf/dal/component";
import StatusIndicatorIcon from "./StatusIndicatorIcon.vue";

const props = defineProps({
  titleCard: { type: Boolean },
  componentId: { type: String as PropType<ComponentId>, required: true },
});

const { theme } = useTheme();

const componentsStore = useComponentsStore();
const component = computed(
  (): FullComponent | undefined =>
    componentsStore.componentsById[props.componentId],
);

const primaryColor = tinycolor(component.value?.color ?? "000000");

// body bg
const bodyBg = computed(() => {
  const bodyBgHsl = primaryColor.toHsl();
  bodyBgHsl.l = theme.value === "dark" ? 0.08 : 0.95;
  return tinycolor(bodyBgHsl);
});

const componentNameRef = ref();
const componentNameTooltip = computed(() => {
  if (
    componentNameRef.value &&
    componentNameRef.value.scrollHeight > componentNameRef.value.offsetHeight
  ) {
    return {
      content: componentNameRef.value.textContent,
      delay: { show: 700, hide: 10 },
    };
  } else {
    return {};
  }
});

const upgradeComponent = async () => {
  componentsStore.setSelectedComponentId(null);
  await componentsStore.UPGRADE_COMPONENT(
    props.componentId,
    component.value?.displayName || "",
  );
};
</script>
