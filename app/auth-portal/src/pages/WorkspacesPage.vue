<template>
  <div v-if="user && user.emailVerified" class="overflow-hidden">
    <div
      class="pb-md flex flex-row gap-sm align-middle items-center justify-between"
    >
      <div>
        <div class="text-lg font-bold pb-sm">Your Workspaces</div>
        <div v-if="featureFlagsStore.CREATE_WORKSPACES">
          From here you can log into any of your workspaces.
        </div>
        <div v-else>
          From here you can log into your local dev instance. Eventually this
          will be where you can manage multiple workspaces, users,
          organizations, etc.
        </div>
      </div>
      <VButton
        v-if="featureFlagsStore.CREATE_WORKSPACES"
        :linkTo="{ name: 'workspace-settings', params: { workspaceId: 'new' } }"
        icon="plus"
        label="Create Workspace"
      />
    </div>
    <div class="mb-sm p-sm border border-neutral-400 rounded-lg">
      If you have questions or need help, join us on
      <a
        class="text-action-500 dark:text-action-300 font-bold hover:underline"
        href="https://discord.gg/system-init"
        target="_blank"
      >
        Discord
      </a>
      or visit our
      <a
        class="text-action-500 dark:text-action-300 font-bold hover:underline"
        href="https://docs.systeminit.com"
        target="_blank"
        >docs site</a
      >
      .
    </div>
    <template v-if="loadWorkspacesReqStatus.isPending">
      <Icon name="loader" />
    </template>
    <template v-else-if="loadWorkspacesReqStatus.isError">
      <ErrorMessage :requestStatus="loadWorkspacesReqStatus" />
    </template>
    <template v-else-if="loadWorkspacesReqStatus.isSuccess">
      <Stack>
        <WorkspaceLinkWidget
          v-for="workspace in sortedWorkspaces(workspaces)"
          :key="workspace.id"
          :workspaceId="workspace.id"
        />
      </Stack>
    </template>
  </div>
  <div v-else>
    You will not be able to use System Initiative until you verify your email.
  </div>
</template>

<script lang="ts" setup>
import { computed, watch } from "vue";
import { Icon, Stack, ErrorMessage, VButton } from "@si/vue-lib/design-system";
import { useHead } from "@vueuse/head";
import { useAuthStore } from "@/store/auth.store";
import { useWorkspacesStore, Workspace } from "@/store/workspaces.store";
import { useFeatureFlagsStore } from "@/store/feature_flags.store";
import WorkspaceLinkWidget from "@/components/WorkspaceLinkWidget.vue";

const authStore = useAuthStore();
const workspacesStore = useWorkspacesStore();
const featureFlagsStore = useFeatureFlagsStore();

const workspaces = computed(() => workspacesStore.workspaces);
// function sortedWorkspaces(workspaces: Workspace[]): Workspace[] {
//   return workspaces.sort((a, b) => {
//     // First, prioritize "SI" instanceEnvType
//     if (a.instanceEnvType === "SI" && b.instanceEnvType !== "SI") {
//       return -1;
//     }
//     if (a.instanceEnvType !== "SI" && b.instanceEnvType === "SI") {
//       return 1;
//     }

//     // If both are "SI" or both are not "SI", sort by displayName
//     return a.displayName.localeCompare(b.displayName);
//   });
// }
function sortedWorkspaces(workspaces: Workspace[]): Workspace[] {
  return workspaces.sort((a, b) => {
    // 1. Sort by isDefault (true comes first)
    if (a.isDefault !== b.isDefault) {
      return a.isDefault ? -1 : 1;
    }

    // 2. Sort by isFavourite (true comes first)
    if (a.isFavourite !== b.isFavourite) {
      return a.isFavourite ? -1 : 1;
    }

    // 3. Sort by instanceEnvType (SI comes first, then REMOTE, then LOCAL)
    if (a.instanceEnvType !== b.instanceEnvType) {
      const envTypeOrder = { SI: 0, PRIVATE: 1, LOCAL: 2 };
      return envTypeOrder[a.instanceEnvType] - envTypeOrder[b.instanceEnvType];
    }

    // 4. If all above are equal, sort by displayName
    return a.displayName.localeCompare(b.displayName);
  });
}

const user = computed(() => authStore.user);

useHead({ title: "Workspaces" });

const loadWorkspacesReqStatus =
  workspacesStore.getRequestStatus("LOAD_WORKSPACES");

function reloadWorkspaces() {
  if (import.meta.env.SSR) return;
  if (!authStore.userIsLoggedIn) return;

  // eslint-disable-next-line @typescript-eslint/no-floating-promises
  workspacesStore.LOAD_WORKSPACES();
}
watch(() => authStore.userIsLoggedIn, reloadWorkspaces, { immediate: true });
</script>
