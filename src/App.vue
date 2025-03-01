<template>
  <n-config-provider>
    <div id="app">
      <n-button type="primary" @click="getHostsEntries">test</n-button>
      <n-data-table
        :columns="columns"
        :data="data"
        :pagination="pagination"
        :bordered="false"
      />
      <n-button @click="getHostsRaw">get raw</n-button>
      {{ raw }}
    </div>
  </n-config-provider>
</template>

<script lang="ts" setup>
import { ref, h } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { NSwitch } from 'naive-ui';

const raw = ref<string>();
async function getHostsRaw() {
  raw.value = await invoke('get_hosts_raw');
}

async function toggleIpStatus(ipAddress: string) {
  try {
    await invoke('toggle_host_ip_status', { ipAddress });
  } catch (e) {
    console.log(JSON.stringify(e));
  }
}

const columns = [
  {
    title: 'ip',
    key: 'ip',
  },
  {
    title: 'hostnames',
    key: 'hostnames',
  },
  {
    title: 'line',
    key: 'line',
  },
  {
    title: 'enabled',
    key: 'enabled',
    render(row: { enabled: boolean; ip: string }) {
      return h(NSwitch, {
        value: row.enabled,
        'onUpdate:value': (value: boolean) => {
          row.enabled = value;
          console.log(row);
          toggleIpStatus(row.ip);
        },
      });
    },
  },
];
const data = ref<string[]>([]);
const pagination = false;
async function getHostsEntries() {
  data.value = await invoke('get_hosts_entries');
}
</script>
