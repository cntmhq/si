<template>
  <div>
    <template
      v-if="fetchDebugReqStatus.isPending || !fetchDebugReqStatus.isRequested"
    >
      <LoadingMessage>Loading debug details...</LoadingMessage>
    </template>
    <template v-else-if="fetchDebugReqStatus.isError">
      <ErrorMessage :requestStatus="fetchDebugReqStatus" />
    </template>
    <template v-else-if="fetchDebugReqStatus.isSuccess && debugData">
      <!-- <SiSearch
        ref="searchRef"
        autoSearch
        disableFilters
        @search="onSearchUpdated"
      /> -->
      <div ref="debugParent" class="border border-neutral-500 m-xs">
        <!-- Component -->
        <TreeNode
          defaultOpen
          alwaysShowArrow
          enableGroupToggle
          label="Component"
          labelClasses="text-lg font-medium border-b border-neutral-200 dark:border-neutral-600"
          childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
          noIndentationOrLeftBorder
        >
          <dl class="border-l-2 p-xs flex flex-col gap-xs">
            <DebugViewItem :data="component.def.id" title="Id" />
            <DebugViewItem
              :data="debugData.schemaVariantId"
              title="Variant Id"
            />
            <DebugViewItem
              :data="debugData.parentId ?? 'NULL'"
              title="Parent Id?"
            />
          </dl>
          <dl class="border-l-2 p-xs flex flex-col gap-xs">
            <DebugViewItem
              v-for="[viewId, geometry] in Object.entries(debugData.geometry)"
              :key="viewId"
              :data="geometry"
              :title="geometryTitle(viewId)"
            />
          </dl>
        </TreeNode>

        <!-- Attributes -->
        <TreeNode
          defaultOpen
          alwaysShowArrow
          enableGroupToggle
          label="Attributes"
          labelClasses="text-lg font-medium border-b border-neutral-200 dark:border-neutral-600"
          childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
          indentationSize="xs"
          leftBorderSize="none"
        >
          <TreeNode
            v-for="attribute in debugData.attributes"
            :key="attribute.path"
            :defaultOpen="false"
            :label="attribute.path"
            alwaysShowArrow
            enableGroupToggle
            labelClasses="text-sm border-l border-b border-neutral-200 dark:border-neutral-600"
            childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
            indentationSize="none"
            leftBorderSize="none"
          >
            <AttributeDebugView :data="attribute" />
          </TreeNode>
        </TreeNode>

        <!-- Input Sockets -->
        <TreeNode
          defaultOpen
          alwaysShowArrow
          enableGroupToggle
          label="Input Sockets"
          labelClasses="text-lg font-medium border-b border-neutral-200 dark:border-neutral-600"
          childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
          indentationSize="xs"
          leftBorderSize="none"
        >
          <TreeNode
            v-for="attribute in debugData.inputSockets"
            :key="attribute.name"
            :defaultOpen="false"
            :label="attribute.name"
            alwaysShowArrow
            enableGroupToggle
            labelClasses="text-sm border-l border-b border-neutral-200 dark:border-neutral-600"
            childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
            indentationSize="none"
            leftBorderSize="none"
          >
            <SocketDebugView :data="attribute" />
          </TreeNode>
        </TreeNode>

        <!-- Output Sockets -->
        <TreeNode
          defaultOpen
          alwaysShowArrow
          enableGroupToggle
          label="Output Sockets"
          labelClasses="text-lg font-medium border-b border-neutral-200 dark:border-neutral-600"
          childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
          indentationSize="xs"
          leftBorderSize="none"
        >
          <TreeNode
            v-for="attribute in debugData.outputSockets"
            :key="attribute.name"
            :defaultOpen="false"
            :label="attribute.name"
            alwaysShowArrow
            enableGroupToggle
            labelClasses="text-sm border-l border-b border-neutral-200 dark:border-neutral-600"
            childrenContainerClasses="border-b border-neutral-200 dark:border-neutral-600"
            indentationSize="none"
            leftBorderSize="none"
          >
            <SocketDebugView :data="attribute" />
          </TreeNode>
        </TreeNode>
      </div>
    </template>
  </div>
</template>

<script lang="ts" setup>
import {
  ErrorMessage,
  LoadingMessage,
  TreeNode,
} from "@si/vue-lib/design-system";
import { computed, onMounted, ref } from "vue";
import { useComponentsStore } from "@/store/components.store";
import { useViewsStore } from "@/store/views.store";
import { ViewId } from "@/api/sdf/dal/views";
import AttributeDebugView from "./AttributeDebugView.vue";
import SocketDebugView from "./SocketDebugView.vue";
import DebugViewItem from "./DebugViewItem.vue";
import {
  DiagramGroupData,
  DiagramNodeData,
} from "../ModelingDiagram/diagram_types";

// const searchRef = ref<InstanceType<typeof SiSearch>>();
const debugParent = ref<InstanceType<typeof Element>>();

const searchString = ref("");

const viewStore = useViewsStore();

const geometryTitle = (viewId: ViewId) => {
  const viewName = viewStore.viewsById[viewId]?.name ?? "unknown";
  return `view: ${viewName}`;
};

function _findChildren(elm: Element) {
  if (elm.tagName === "DD" || elm.tagName === "DT")
    if (
      elm.textContent?.toLowerCase().includes(searchString.value.toLowerCase())
    ) {
      elm.classList.add("search-found");
    } else {
      elm.classList.remove("search-found");
    }

  for (const child of elm.children) _findChildren(child);
}

// function onSearchUpdated(newFilterString: string) {
//   searchString.value = newFilterString.trim();
//   if (!searchString.value) {
//     for (const elm of document.getElementsByClassName("search-found")) {
//       elm.classList.remove("search-found");
//     }
//   } else {
//     if (debugParent.value) {
//       for (const child of debugParent.value.children) {
//         _findChildren(child);
//       }
//       const found = document.getElementsByClassName("search-found");
//       if (found.length > 0)
//         found[0]?.scrollIntoView({ behavior: "smooth", block: "nearest" });
//     }
//   }
// }

const componentsStore = useComponentsStore();

const debugData = computed(
  () => componentsStore.debugDataByComponentId[props.component.def.id],
);

const props = defineProps<{
  component: DiagramNodeData | DiagramGroupData;
}>();

const fetchDebugReqStatus = componentsStore.getRequestStatus(
  "FETCH_COMPONENT_DEBUG_VIEW",
  computed(() => props.component.def.id),
);

onMounted(() => {
  componentsStore.FETCH_COMPONENT_DEBUG_VIEW(props.component.def.id);
});
</script>

<style type="less">
.search-found {
  background-color: rgba(255, 255, 0, 0.5);
}
</style>
