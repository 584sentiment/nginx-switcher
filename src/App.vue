<template>
    <n-config-provider>
        <div id="app">
            <n-data-table
                :columns="columns"
                :data="data"
                :pagination="pagination"
                :bordered="false"
            />
        </div>
    </n-config-provider>
</template>

<script lang="ts" setup>
import { ref, h, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { NSwitch } from "naive-ui";
import { readTextFile } from "@tauri-apps/plugin-fs";

// 1. 获取hosts文件的path
async function getHostsPath(): Promise<string> {
    return await invoke("get_hosts_path");
}

// 2. 获取hosts文件的内容
async function getHostsRaw(path: string) {
    return await readTextFile(path);
}

// 3. 格式化hosts
let raw = "";
const data = ref<string[]>([]);
async function getHostsData() {
    const host_path = await getHostsPath();
    raw = await getHostsRaw(host_path);
    data.value = await invoke("parse_hosts_content", { content: raw });
}

// 4. 切换ip的启用状态
async function toggleIpStatus(status: Boolean, line: string) {
    try {
        const newLine = status ? line.replace("#", "") : `# ${line}`;
        const newHosts = raw.replace(line, newLine);
        await invoke("modify_hosts_file", { content: newHosts });
        await getHostsData();
    } catch (e) {
        console.log(JSON.stringify(e));
    }
}

const columns = [
    {
        title: "ip",
        key: "ip",
    },
    {
        title: "hostnames",
        key: "hostnames",
    },
    {
        title: "line",
        key: "line",
    },
    {
        title: "enabled",
        key: "enabled",
        render(row: { enabled: boolean; ip: string; line: string }) {
            return h(NSwitch, {
                value: row.enabled,
                "onUpdate:value": (value: boolean) => {
                    console.log(row);
                    toggleIpStatus(value, row.line);
                },
            });
        },
    },
];
const pagination = false;
onMounted(() => {
    getHostsData();
});
</script>
