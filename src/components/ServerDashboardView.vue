<template>
  <div class="h-screen w-screen flex flex-col bg-[#050914] text-cyan-50 font-mono p-4 relative overflow-hidden bg-[radial-gradient(ellipse_at_top_right,_var(--tw-gradient-stops))] from-blue-900/20 via-[#050914] to-[#050914]">
    <!-- Animated background grid -->
    <div class="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGcgb3BhY2l0eT0iMC4wNSI+PHBhdGggZD0iTTAgNjBMMjAgNjBMMjAgMEwwIDBaIiBmaWxsPSIjMThBMEZCIi8+PHBhdGggZD0iTTYwIDBMMCAwTDAgMjBMNjAgMjBaIiBmaWxsPSIjMThBMEZCIi8+PC9nPjwvc3ZnPg==')] opacity-10 pointer-events-none" />

    <!-- Top Bar -->
    <div class="flex items-center justify-between mb-4 pb-4 border-b border-cyan-900/50 shrink-0 relative z-10">
      <div class="flex items-center gap-4">
        <div class="relative">
          <div
            class="absolute inset-0 bg-cyan-500 blur-md opacity-20"
            :class="{'animate-pulse': isRunning}"
          />
          <div
            class="w-12 h-12 rounded-xl flex items-center justify-center border"
            :class="isRunning ? 'bg-cyan-950 border-cyan-500/50' : 'bg-slate-900 border-slate-700'"
          >
            <Server
              class="w-6 h-6 transition-colors"
              :class="isRunning ? 'text-cyan-400' : 'text-slate-500'"
            />
          </div>
        </div>
        <div>
          <h1 class="text-2xl font-black tracking-widest uppercase bg-clip-text text-transparent bg-gradient-to-r from-cyan-400 to-blue-500">
            {{ t('role.dashboard_title') }}
          </h1>
          <p
            class="text-xs tracking-widest uppercase flex items-center gap-2 mt-0.5"
            :class="isRunning ? 'text-cyan-400' : 'text-slate-500'"
          >
            <span
              class="w-1.5 h-1.5 rounded-full"
              :class="isRunning ? 'bg-cyan-400 animate-ping' : 'bg-slate-500'"
            />
            {{ isRunning ? t('role.dashboard_running', { port: serverPort }) : t('role.server_stopped') }}
          </p>
        </div>
      </div>
      <div class="flex gap-3">
        <button
          class="flex items-center gap-2 px-4 py-2 bg-slate-900/50 hover:bg-slate-800 rounded-lg border border-slate-700/50 text-xs text-slate-300 backdrop-blur-md transition-all"
          @click="cycleLanguage"
        >
          <Globe class="w-4 h-4 text-cyan-500" /> {{ currentLangLabel }}
        </button>
        <button
          class="flex items-center gap-2 px-4 py-2 bg-blue-900/30 text-blue-300 hover:bg-blue-800/40 rounded-lg border border-blue-500/30 text-xs font-bold backdrop-blur-md transition-all"
          @click="openNewClient"
        >
          <Monitor class="w-4 h-4" /> {{ t('role.dashboard_open_client') }}
        </button>
        <button
          class="flex items-center gap-2 px-4 py-2 bg-red-950/30 hover:bg-red-900/40 text-red-300 rounded-lg border border-red-900/50 text-xs font-bold backdrop-blur-md transition-all"
          @click="stopAndExit"
        >
          <LogOut class="w-4 h-4" /> {{ t('role.back') }}
        </button>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 flex gap-4 overflow-hidden relative z-10">
      <!-- Left Panel: Server Config + Radar Map -->
      <div class="w-80 flex flex-col gap-4 shrink-0 overflow-hidden">
        <!-- Connection Info / Start -->
        <div class="bg-slate-900/40 backdrop-blur-xl rounded-2xl p-5 border border-cyan-900/30 shadow-[0_0_15px_rgba(8,145,178,0.05)]">
          <h2 class="text-[10px] text-cyan-500 mb-4 uppercase tracking-[0.2em] font-bold flex items-center gap-2">
            <Activity class="w-3 h-3" />
            {{ t('role.dashboard_conn_info') }}
          </h2>
          <div
            v-if="!isRunning"
            class="space-y-3"
          >
            <div>
              <label class="text-[10px] text-slate-400 block mb-1 uppercase tracking-wider">BIND IP</label>
              <input
                v-model="serverHost"
                class="w-full bg-slate-950/50 border border-slate-700 rounded-lg px-3 py-2 text-xs outline-none focus:border-cyan-500 text-cyan-100 font-mono transition-colors"
              >
            </div>
            <div>
              <label class="text-[10px] text-slate-400 block mb-1 uppercase tracking-wider">PORT</label>
              <input
                v-model.number="serverPort"
                type="number"
                class="w-full bg-slate-950/50 border border-slate-700 rounded-lg px-3 py-2 text-xs outline-none focus:border-cyan-500 text-cyan-100 font-mono transition-colors"
              >
            </div>
            <button
              class="w-full mt-4 py-2.5 bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 text-white font-bold rounded-lg text-xs flex justify-center items-center gap-2 shadow-[0_0_15px_rgba(8,145,178,0.3)] transition-all"
              @click="startLocalServer"
            >
              <Play
                class="w-4 h-4"
                fill="currentColor"
              /> {{ t('role.start_server_btn') }}
            </button>
          </div>
          <div
            v-else
            class="space-y-4"
          >
            <div class="text-2xl font-black text-white tracking-widest bg-slate-950/50 p-3 rounded-lg border border-cyan-900/50 text-center shadow-inner">
              <span class="text-cyan-500">{{ serverHost }}</span>:<span class="text-blue-400">{{ serverPort }}</span>
            </div>
            <p class="text-xs text-cyan-600/70 text-center animate-pulse uppercase tracking-[0.2em]">
              {{ t('role.dashboard_listening') }}
            </p>
            <button
              class="w-full py-2.5 bg-red-950/40 text-red-400 hover:bg-red-900/50 font-bold rounded-lg text-xs border border-red-900/50 flex justify-center items-center gap-2 transition-all"
              @click="stopLocalServer"
            >
              <Square
                class="w-4 h-4"
                fill="currentColor"
              /> {{ t('role.stop_server_btn') }}
            </button>
          </div>
        </div>

        <!-- Radar Map (Online Clients) -->
        <div class="bg-slate-900/40 backdrop-blur-xl rounded-2xl p-5 border border-cyan-900/30 flex-1 overflow-hidden flex flex-col relative group">
          <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))] from-cyan-900/10 to-transparent pointer-events-none" />
          
          <div class="flex justify-between items-center mb-4 relative z-10">
            <h2 class="text-[10px] text-cyan-500 uppercase tracking-[0.2em] font-bold flex items-center gap-2">
              <Radar
                class="w-3 h-3"
                :class="{'animate-spin-slow': isRunning}"
              />
              PLAYER RADAR
            </h2>
            <span class="text-xs font-black text-cyan-300 bg-cyan-950/50 px-2 py-0.5 rounded border border-cyan-800/50">{{ onlineClients.length }} ONLINE</span>
          </div>

          <div
            v-if="onlineClients.length > 0"
            class="flex-1 overflow-y-auto space-y-2 relative z-10 custom-scrollbar pr-1"
          >
            <div
              v-for="c in onlineClients"
              :key="c.user_id"
              class="flex items-center gap-3 p-3 bg-slate-950/60 rounded-xl border border-cyan-900/40 hover:border-cyan-400/50 hover:bg-cyan-950/30 cursor-pointer transition-all hover:shadow-[0_0_10px_rgba(34,211,238,0.2)]"
              @click="selectUser(c.user_id)"
            >
              <div class="relative">
                <div class="absolute inset-0 bg-cyan-400 blur-sm opacity-50 animate-pulse" />
                <img
                  :src="c.avatar_url"
                  class="w-10 h-10 rounded-full border-2 border-cyan-400/50 object-cover relative z-10"
                  @error="(e) => (e.target as HTMLImageElement).src='https://assets.vrchat.com/www/images/default_avatar.png'"
                >
              </div>
              <div class="min-w-0 flex-1">
                <div class="text-sm font-bold text-white truncate drop-shadow-md">
                  {{ c.display_name }}
                </div>
                <div class="text-[10px] text-cyan-600 truncate font-mono mt-0.5">
                  {{ c.ip_address }}
                </div>
              </div>
              <div class="text-cyan-500/50 hover:text-cyan-300">
                <ChevronRight class="w-4 h-4" />
              </div>
            </div>
          </div>
          <div
            v-else
            class="flex-1 flex flex-col items-center justify-center text-cyan-800/50 text-xs text-center relative z-10 gap-3"
          >
            <Radar class="w-12 h-12 opacity-20" />
            NO SIGNALS DETECTED
          </div>
          
          <!-- Radar Sweep Animation overlay -->
          <div
            v-if="isRunning && onlineClients.length === 0"
            class="absolute inset-0 pointer-events-none overflow-hidden rounded-2xl"
          >
            <div
              class="w-[200%] h-[200%] absolute top-[-50%] left-[-50%] border-t border-cyan-500/20 rounded-full animate-radar-sweep"
              style="background: conic-gradient(from 0deg, transparent 0deg, transparent 90deg, rgba(6, 182, 212, 0.1) 180deg);"
            />
          </div>
        </div>
      </div>

      <!-- Right Panel: Tabs -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Tab Buttons -->
        <div class="flex gap-2 mb-4 shrink-0 bg-slate-900/40 backdrop-blur-xl p-1.5 rounded-xl border border-cyan-900/30 inline-flex w-fit shadow-[0_0_15px_rgba(0,0,0,0.5)]">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            class="px-5 py-2 rounded-lg text-xs font-bold transition-all uppercase tracking-wider relative overflow-hidden"
            :class="activeTab === tab.key ? 'text-white' : 'text-slate-500 hover:text-slate-300'"
            @click="activeTab = tab.key"
          >
            <div
              v-if="activeTab === tab.key"
              class="absolute inset-0 bg-gradient-to-r from-cyan-600/80 to-blue-600/80 -z-10"
            />
            {{ tab.label }}
          </button>
        </div>

        <!-- Tab: Terminal Logs -->
        <div
          v-show="activeTab === 'logs'"
          class="flex-1 bg-[#02040A]/80 backdrop-blur-xl rounded-2xl border border-cyan-900/30 p-4 flex flex-col overflow-hidden relative shadow-[inset_0_0_20px_rgba(0,0,0,0.8)]"
        >
          <div class="flex justify-between items-center mb-3">
            <span class="text-[10px] text-cyan-600 uppercase tracking-widest font-bold flex items-center gap-2"><Terminal class="w-3 h-3" /> SYSTEM LOGS</span>
            <button
              class="text-[10px] text-slate-500 hover:text-cyan-400 uppercase tracking-wider border border-slate-700 hover:border-cyan-800 px-2 py-1 rounded transition-colors"
              @click="logs = []"
            >
              {{ t('role.dashboard_clear_logs') }}
            </button>
          </div>
          <div
            ref="logContainer"
            class="flex-1 overflow-y-auto text-[11px] font-mono pr-2 custom-scrollbar"
          >
            <div
              v-for="(log, idx) in logs"
              :key="idx"
              class="mb-1 leading-relaxed break-all flex gap-3 hover:bg-white/5 px-2 py-0.5 rounded transition-colors"
            >
              <span class="text-slate-600 shrink-0 select-none">[{{ log.time }}]</span>
              <span :class="{'text-cyan-300': log.level==='INFO','text-red-400': log.level==='ERROR','text-indigo-400': log.level==='WARN','text-emerald-400': log.level==='SUCCESS'}">{{ log.content }}</span>
            </div>
            <div
              v-if="logs.length===0"
              class="text-slate-700 mt-10 text-center uppercase tracking-widest opacity-50 flex flex-col items-center gap-2"
            >
              <Terminal class="w-8 h-8" /> AWAITING INPUT...
            </div>
          </div>
        </div>

        <!-- Tab: User Management -->
        <div
          v-show="activeTab === 'users'"
          class="flex-1 bg-slate-900/40 backdrop-blur-xl rounded-2xl border border-cyan-900/30 p-1 flex flex-col overflow-hidden"
        >
          <div class="flex-1 overflow-y-auto custom-scrollbar">
            <table
              v-if="allUsers.length > 0"
              class="w-full text-xs"
            >
              <thead class="sticky top-0 bg-slate-900/90 backdrop-blur-md z-10 shadow-sm">
                <tr class="text-cyan-600/70 text-left text-[10px] uppercase tracking-widest">
                  <th class="py-3 px-4 rounded-tl-xl">
                    {{ t('role.user_name') }}
                  </th>
                  <th class="py-3 px-2">
                    {{ t('role.status') }}
                  </th>
                  <th class="py-3 px-2">
                    {{ t('role.login_count') }}
                  </th>
                  <th class="py-3 px-2">
                    {{ t('role.role') }}
                  </th>
                  <th class="py-3 px-4 text-right rounded-tr-xl">
                    {{ t('role.action') }}
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-cyan-900/20">
                <tr
                  v-for="u in allUsers"
                  :key="u.user_id"
                  class="hover:bg-cyan-950/20 transition-colors group"
                >
                  <td class="py-3 px-4">
                    <div class="flex items-center gap-3">
                      <img
                        :src="u.avatar_url"
                        class="w-8 h-8 rounded-lg object-cover border border-cyan-900/50"
                        @error="(e) => (e.target as HTMLImageElement).src='https://assets.vrchat.com/www/images/default_avatar.png'"
                      >
                      <div>
                        <div class="font-bold text-slate-200 group-hover:text-cyan-300 transition-colors">
                          {{ u.display_name }}
                        </div>
                        <div class="text-[9px] text-slate-500 font-mono">
                          {{ u.user_id }}
                        </div>
                      </div>
                    </div>
                  </td>
                  <td class="py-3 px-2">
                    <div v-if="banMap[u.user_id]">
                      <span class="px-2 py-0.5 bg-red-500/20 text-red-400 border border-red-500/30 rounded text-[10px] uppercase tracking-wider font-bold">{{ t('role.ban') }}</span>
                      <div class="text-[9px] text-red-400/70 mt-1 whitespace-nowrap font-mono">
                        <span v-if="banMap[u.user_id].duration_hours">{{ banMap[u.user_id].duration_hours }}{{ t('role.hours') }}</span>
                        <span v-else>{{ t('role.permanent') }}</span>
                      </div>
                    </div>
                    <div v-else-if="freezeMap[u.user_id]">
                      <span class="px-2 py-0.5 bg-blue-500/20 text-blue-400 border border-blue-500/30 rounded text-[10px] uppercase tracking-wider font-bold">{{ t('role.freeze') }}</span>
                    </div>
                    <span
                      v-else-if="u.is_online"
                      class="px-2 py-0.5 bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 rounded text-[10px] uppercase tracking-wider font-bold shadow-[0_0_10px_rgba(34,211,238,0.2)]"
                    >{{ t('role.online') }}</span>
                    <span
                      v-else
                      class="px-2 py-0.5 bg-slate-800 text-slate-500 border border-slate-700 rounded text-[10px] uppercase tracking-wider font-bold"
                    >{{ t('role.offline') }}</span>
                  </td>
                  <td class="py-3 px-2 text-slate-400 font-mono text-[10px]">
                    {{ u.login_count }}
                  </td>
                  <td class="py-3 px-2">
                    <select
                      v-model="u.role_id"
                      class="bg-slate-900/80 border border-slate-700 rounded-lg text-[10px] px-2 py-1 outline-none focus:border-cyan-500 text-cyan-100 uppercase tracking-wider cursor-pointer"
                      @change="setUserRole(u.user_id, u.role_id)"
                    >
                      <option :value="null">
                        {{ t('role.default_role') }}
                      </option>
                      <option
                        v-for="r in allRoles"
                        :key="r.role_id"
                        :value="r.role_id"
                      >
                        {{ r.role_name }}
                      </option>
                    </select>
                  </td>
                  <td class="py-3 px-4 text-right">
                    <div class="flex gap-1.5 justify-end flex-wrap">
                      <button
                        v-if="u.is_online"
                        class="px-2 py-1 bg-indigo-500/10 text-indigo-500 border border-slate-1000/30 hover:bg-indigo-500/20 hover:border-slate-1000/50 rounded text-[9px] uppercase font-bold tracking-wider transition-colors"
                        @click="kickUser(u.user_id)"
                      >
                        {{ t('role.kick') }}
                      </button>
                      <button
                        v-if="!banMap[u.user_id]"
                        class="px-2 py-1 bg-red-500/10 text-red-500 border border-red-500/30 hover:bg-red-500/20 hover:border-red-500/50 rounded text-[9px] uppercase font-bold tracking-wider transition-colors"
                        @click="openBanDialog(u)"
                      >
                        {{ t('role.ban') }}
                      </button>
                      <button
                        v-else
                        class="px-2 py-1 bg-green-500/10 text-green-500 border border-green-500/30 hover:bg-green-500/20 hover:border-green-500/50 rounded text-[9px] uppercase font-bold tracking-wider transition-colors"
                        @click="unbanUser(u.user_id)"
                      >
                        {{ t('role.unban') }}
                      </button>
                      <button
                        v-if="!freezeMap[u.user_id]"
                        class="px-2 py-1 bg-blue-500/10 text-blue-500 border border-blue-500/30 hover:bg-blue-500/20 hover:border-blue-500/50 rounded text-[9px] uppercase font-bold tracking-wider transition-colors"
                        @click="openFreezeDialog(u)"
                      >
                        {{ t('role.freeze') }}
                      </button>
                      <button
                        v-else
                        class="px-2 py-1 bg-cyan-500/10 text-cyan-500 border border-cyan-500/30 hover:bg-cyan-500/20 hover:border-cyan-500/50 rounded text-[9px] uppercase font-bold tracking-wider transition-colors"
                        @click="unfreezeUser(u.user_id)"
                      >
                        {{ t('role.unfreeze') }}
                      </button>
                      <button
                        class="px-2 py-1 bg-slate-800 text-slate-400 border border-slate-700 hover:bg-slate-700 hover:text-white rounded text-[9px] uppercase font-bold tracking-wider transition-colors"
                        @click="removeUser(u.user_id)"
                      >
                        <Trash2 class="w-3 h-3" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
            <div
              v-else
              class="flex-1 flex flex-col items-center justify-center text-slate-600 text-xs h-full gap-3 uppercase tracking-widest"
            >
              <Users class="w-10 h-10 opacity-30" />
              {{ t('role.no_user_records') }}
            </div>
          </div>
        </div>

        <!-- Tab: Roles -->
        <div
          v-show="activeTab === 'features'"
          class="flex-1 bg-slate-900/40 backdrop-blur-xl rounded-2xl border border-cyan-900/30 p-1 flex overflow-hidden"
        >
          <!-- Roles List -->
          <div class="w-1/3 flex flex-col border-r border-cyan-900/30 p-3 bg-slate-950/30">
            <div class="flex justify-between items-center mb-4 shrink-0">
              <h3 class="text-[10px] font-bold text-cyan-500 uppercase tracking-widest flex items-center gap-2">
                <Shield class="w-3 h-3" /> {{ t('role.dashboard_roles_list') || 'ROLES LIST' }}
              </h3>
              <button
                class="px-2 py-1 bg-cyan-500/20 text-cyan-400 hover:bg-cyan-500/40 border border-cyan-500/30 rounded text-[10px] font-bold transition-colors"
                @click="createNewRole"
              >
                +
              </button>
            </div>
            <div class="flex-1 overflow-y-auto space-y-2 custom-scrollbar pr-1">
              <div
                v-for="r in allRoles"
                :key="r.role_id"
                class="p-3 rounded-xl border cursor-pointer flex justify-between items-center transition-all group"
                :class="selectedRole?.role_id === r.role_id ? 'bg-cyan-900/30 border-cyan-500/50 shadow-[0_0_15px_rgba(6,182,212,0.1)]' : 'bg-slate-900/50 border-slate-800 text-slate-400 hover:border-cyan-900/50'"
                @click="selectRole(r)"
              >
                <div
                  class="text-xs font-bold"
                  :class="selectedRole?.role_id === r.role_id ? 'text-cyan-300' : 'text-slate-300'"
                >
                  {{ r.role_name }} 
                  <span
                    v-if="r.is_default"
                    class="text-[8px] uppercase tracking-wider bg-slate-800 text-slate-400 px-1.5 py-0.5 rounded ml-2 font-mono"
                  >{{ t('role.default') }}</span>
                </div>
                <div
                  v-if="selectedRole?.role_id === r.role_id"
                  class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity"
                >
                  <button
                    v-if="!r.is_default"
                    class="text-slate-400 hover:text-indigo-400 transition-colors"
                    :title="t('role.set_default')"
                    @click.stop="setDefaultRole(r.role_id)"
                  >
                    <Star class="w-3.5 h-3.5" />
                  </button>
                  <button
                    v-if="!r.is_default"
                    class="text-slate-400 hover:text-red-400 transition-colors"
                    :title="t('role.delete')"
                    @click.stop="deleteRole(r.role_id)"
                  >
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            </div>
          </div>
          <!-- Role Editor -->
          <div
            v-if="selectedRole"
            class="flex-1 flex flex-col overflow-y-auto p-4 custom-scrollbar"
          >
            <div class="mb-5 shrink-0 bg-slate-900/50 p-4 rounded-xl border border-cyan-900/30">
              <label class="text-[10px] text-cyan-600 font-bold block mb-2 uppercase tracking-widest">{{ t('role.role_name') }}</label>
              <input
                v-model="selectedRole.role_name"
                class="w-full bg-[#02040A] border border-cyan-900/50 rounded-lg px-3 py-2 text-sm outline-none focus:border-cyan-500 text-cyan-100 font-bold transition-colors shadow-inner"
              >
            </div>
            
            <div class="space-y-6 flex-1">
              <div>
                <h3 class="text-[10px] font-bold text-slate-400 mb-3 uppercase tracking-widest border-b border-slate-800 pb-1">
                  {{ t('role.feature_menus') }}
                </h3>
                <div class="grid grid-cols-3 gap-2">
                  <label
                    v-for="(enabled, key) in selectedRole.features.menus"
                    :key="key"
                    class="flex items-center gap-3 p-2 bg-slate-900/50 rounded-lg border border-slate-800 hover:border-cyan-900/50 cursor-pointer text-xs group transition-colors"
                    :class="{'bg-cyan-950/30 border-cyan-900/60': enabled}"
                  >
                    <div
                      class="w-4 h-4 rounded border flex items-center justify-center transition-colors"
                      :class="enabled ? 'bg-cyan-500 border-cyan-400' : 'bg-slate-950 border-slate-700'"
                    >
                      <Check
                        v-if="enabled"
                        class="w-3 h-3 text-[#050914] font-bold"
                        stroke-width="3"
                      />
                    </div>
                    <input
                      v-model="selectedRole.features.menus[key]"
                      type="checkbox"
                      class="hidden"
                    >
                    <span class="text-slate-300 font-medium group-hover:text-cyan-100 transition-colors">{{ t('sidebar.' + key) || key }}</span>
                  </label>
                </div>
              </div>
              
              <div>
                <h3 class="text-[10px] font-bold text-slate-400 mb-3 uppercase tracking-widest border-b border-slate-800 pb-1">
                  {{ t('role.feature_modes') }}
                </h3>
                <div class="grid grid-cols-2 gap-2">
                  <label
                    v-for="(enabled, key) in selectedRole.features.modes"
                    :key="key"
                    class="flex items-center gap-3 p-2 bg-slate-900/50 rounded-lg border border-slate-800 hover:border-cyan-900/50 cursor-pointer text-xs group transition-colors"
                    :class="{'bg-cyan-950/30 border-cyan-900/60': enabled}"
                  >
                    <div
                      class="w-4 h-4 rounded border flex items-center justify-center transition-colors"
                      :class="enabled ? 'bg-cyan-500 border-cyan-400' : 'bg-slate-950 border-slate-700'"
                    >
                      <Check
                        v-if="enabled"
                        class="w-3 h-3 text-[#050914] font-bold"
                        stroke-width="3"
                      />
                    </div>
                    <input
                      v-model="selectedRole.features.modes[key]"
                      type="checkbox"
                      class="hidden"
                    >
                    <span class="text-slate-300 font-medium group-hover:text-cyan-100 transition-colors uppercase tracking-wider">{{ t('role.mode_' + key) || (key === 'pc' ? 'PC Desktop' : key === 'vr' ? 'VR Overlay' : key) }}</span>
                  </label>
                </div>
              </div>
              
              <div>
                <h3 class="text-[10px] font-bold text-slate-400 mb-3 uppercase tracking-widest border-b border-slate-800 pb-1">
                  {{ t('role.feature_themes') }}
                </h3>
                <div class="grid grid-cols-3 gap-2">
                  <label
                    v-for="(enabled, key) in selectedRole.features.themes"
                    :key="key"
                    class="flex items-center gap-3 p-2 bg-slate-900/50 rounded-lg border border-slate-800 hover:border-cyan-900/50 cursor-pointer text-xs group transition-colors"
                    :class="{'bg-cyan-950/30 border-cyan-900/60': enabled}"
                  >
                    <div
                      class="w-4 h-4 rounded border flex items-center justify-center transition-colors"
                      :class="enabled ? 'bg-cyan-500 border-cyan-400' : 'bg-slate-950 border-slate-700'"
                    >
                      <Check
                        v-if="enabled"
                        class="w-3 h-3 text-[#050914] font-bold"
                        stroke-width="3"
                      />
                    </div>
                    <input
                      v-model="selectedRole.features.themes[key]"
                      type="checkbox"
                      class="hidden"
                    >
                    <span class="text-slate-300 font-medium group-hover:text-cyan-100 transition-colors uppercase tracking-wider">{{ t('role.theme_' + key) || (key.charAt(0).toUpperCase() + key.slice(1)) }}</span>
                  </label>
                </div>
              </div>
            </div>
            
            <div class="mt-6 pt-4 border-t border-cyan-900/30 shrink-0">
              <button
                class="w-full py-2.5 bg-gradient-to-r from-cyan-600 to-blue-600 hover:from-cyan-500 hover:to-blue-500 text-white font-black rounded-xl text-xs uppercase tracking-widest shadow-[0_0_15px_rgba(8,145,178,0.3)] transition-all flex items-center justify-center gap-2"
                @click="saveRole"
              >
                <Save class="w-4 h-4" /> {{ t('role.save_role') }}
              </button>
            </div>
          </div>
          <div
            v-else
            class="flex-1 flex flex-col items-center justify-center text-slate-600 text-xs gap-3 uppercase tracking-widest font-bold"
          >
            <Shield class="w-12 h-12 opacity-20" />
            {{ t('role.select_or_create_role') }}
          </div>
        </div>
      </div>
    </div>

    <!-- Ban Dialog -->
    <div
      v-if="showBanDialog"
      class="fixed inset-0 bg-[#050914]/80 backdrop-blur-md flex items-center justify-center z-[999]"
      @click.self="showBanDialog=false"
    >
      <div class="bg-slate-900/90 rounded-2xl p-6 w-96 border border-red-900/50 shadow-[0_0_30px_rgba(220,38,38,0.2)]">
        <h3 class="text-sm font-black mb-4 text-red-500 uppercase tracking-widest flex items-center gap-2">
          <ShieldAlert class="w-4 h-4" /> {{ t('role.ban_user') }}
        </h3>
        <p class="text-white font-bold mb-4 bg-slate-950 p-2 rounded-lg border border-slate-800">
          {{ dialogUser?.display_name }}
        </p>
        <div class="space-y-4">
          <div>
            <label class="text-[10px] text-slate-400 block mb-1 uppercase tracking-wider">{{ t('role.ban_reason') }}</label>
            <input
              v-model="banReason"
              class="w-full bg-[#02040A] border border-red-900/30 rounded-lg px-3 py-2 text-sm outline-none focus:border-red-500 text-white"
              :placeholder="t('role.ban_reason_ph')"
            >
          </div>
          <div>
            <label class="text-[10px] text-slate-400 block mb-1 uppercase tracking-wider">{{ t('role.ban_duration') }}</label>
            <input
              v-model.number="banDuration"
              type="number"
              class="w-full bg-[#02040A] border border-red-900/30 rounded-lg px-3 py-2 text-sm outline-none focus:border-red-500 text-white"
              :placeholder="t('role.ban_duration_ph')"
            >
          </div>
          <div class="flex gap-3 justify-end mt-6 pt-4 border-t border-slate-800">
            <button
              class="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg text-xs font-bold text-slate-300 uppercase tracking-wider transition-colors"
              @click="showBanDialog=false"
            >
              {{ t('role.cancel') }}
            </button>
            <button
              class="px-4 py-2 bg-red-600 hover:bg-red-500 text-white font-black rounded-lg text-xs uppercase tracking-widest shadow-[0_0_15px_rgba(220,38,38,0.4)] transition-all"
              @click="confirmBan"
            >
              {{ t('role.confirm_ban') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Freeze Dialog -->
    <div
      v-if="showFreezeDialog"
      class="fixed inset-0 bg-[#050914]/80 backdrop-blur-md flex items-center justify-center z-[999]"
      @click.self="showFreezeDialog=false"
    >
      <div class="bg-slate-900/90 rounded-2xl p-6 w-96 border border-blue-900/50 shadow-[0_0_30px_rgba(37,99,235,0.2)]">
        <h3 class="text-sm font-black mb-4 text-blue-400 uppercase tracking-widest flex items-center gap-2">
          <Snowflake class="w-4 h-4" /> {{ t('role.freeze_user') }}
        </h3>
        <p class="text-white font-bold mb-4 bg-slate-950 p-2 rounded-lg border border-slate-800">
          {{ dialogUser?.display_name }}
        </p>
        <div class="space-y-4">
          <div>
            <label class="text-[10px] text-slate-400 block mb-1 uppercase tracking-wider">{{ t('role.freeze_reason') }}</label>
            <input
              v-model="freezeReason"
              class="w-full bg-[#02040A] border border-blue-900/30 rounded-lg px-3 py-2 text-sm outline-none focus:border-blue-500 text-white"
              :placeholder="t('role.freeze_reason_ph')"
            >
          </div>
          <div class="flex gap-3 justify-end mt-6 pt-4 border-t border-slate-800">
            <button
              class="px-4 py-2 bg-slate-800 hover:bg-slate-700 rounded-lg text-xs font-bold text-slate-300 uppercase tracking-wider transition-colors"
              @click="showFreezeDialog=false"
            >
              {{ t('role.cancel') }}
            </button>
            <button
              class="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white font-black rounded-lg text-xs uppercase tracking-widest shadow-[0_0_15px_rgba(37,99,235,0.4)] transition-all"
              @click="confirmFreeze"
            >
              {{ t('role.confirm_freeze') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Toast -->
    <div
      v-if="toastMessage"
      class="fixed top-8 left-1/2 transform -translate-x-1/2 px-6 py-3 bg-cyan-900/90 backdrop-blur-md border border-cyan-400/50 text-cyan-50 rounded-xl shadow-[0_0_20px_rgba(6,182,212,0.5)] z-[1000] text-sm font-black tracking-widest transition-all uppercase flex items-center gap-2"
    >
      <div class="w-2 h-2 bg-cyan-400 rounded-full animate-pulse" />
      {{ toastMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, computed, reactive } from 'vue';
import { Server, Globe, Monitor, LogOut, Play, Square, Activity, Radar, ChevronRight, Terminal, Users, Shield, Star, Trash2, ShieldAlert, Snowflake, Check, Save } from 'lucide-vue-next';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';
import { SysApi, DbApi, VrcApi } from '../api';
import { useStorage } from '@vueuse/core';

const { t, locale } = useI18n();
const langMap: Record<string, string> = { 'zh-CN': '简体中文', 'en-US': 'English', 'ja-JP': '日本語' };
const currentLangLabel = computed(() => langMap[locale.value] || 'Language');
const cycleLanguage = () => {
  const keys = Object.keys(langMap);
  locale.value = keys[(keys.indexOf(locale.value) + 1) % keys.length];
  localStorage.setItem('vrcdog-locale', locale.value);
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(locale.value) }).catch(() => {});
};

const emit = defineEmits(['exit']);
const serverHost = useStorage('vrc_dashboard_host', '0.0.0.0');
const serverPort = useStorage('vrc_dashboard_port', 11451);
const isRunning = ref(false);
const activeTab = ref('logs');
const tabs = computed(() => [
  { key: 'logs', label: t('role.dashboard_logs_tab') || '终端日志' },
  { key: 'users', label: t('role.dashboard_users_tab') || '用户管理' },
  { key: 'features', label: t('role.dashboard_roles_tab') || '角色与权限' },
]);

const toastMessage = ref('');
const showToast = (msg: string) => {
  toastMessage.value = msg;
  setTimeout(() => toastMessage.value = '', 3000);
};

// === Server URL helper ===
const serverUrl = computed(() => {
  const h = serverHost.value === '0.0.0.0' ? '127.0.0.1' : serverHost.value;
  return `http://${h}:${serverPort.value}`;
});

// === Logs ===
interface LogEntry { time: string; level: string; content: string; }
const logs = ref<LogEntry[]>([]);
const logContainer = ref<HTMLElement | null>(null);
let unlistenLog: UnlistenFn | null = null;
let unlistenClients: UnlistenFn | null = null;

const addLog = (content: string) => {
  const time = new Date().toTimeString().split(' ')[0];
  let level = 'INFO', clean = content;
  if (content.startsWith('[ERROR]')) { level = 'ERROR'; clean = content.substring(7).trim(); }
  else if (content.startsWith('[WARN]')) { level = 'WARN'; clean = content.substring(6).trim(); }
  else if (content.startsWith('[INFO]')) { level = 'INFO'; clean = content.substring(6).trim(); }
  logs.value.push({ time, level, content: clean });
  if (logs.value.length > 500) logs.value.shift();
  nextTick(() => { if (logContainer.value) logContainer.value.scrollTop = logContainer.value.scrollHeight; });
};

// === Online Clients ===
interface ClientInfo { user_id: string; display_name: string; avatar_url: string; ip_address: string; connected_at: string; last_heartbeat: string; }
interface UserRecord { user_id: string; display_name: string; avatar_url: string; first_seen: string; last_seen: string; login_count: number; is_online: boolean; role_id: string | null; }
interface BanInfo { user_id: string; reason: string; banned_at: string; duration_hours: number | null; expires_at: string | null; }
interface FreezeInfo { user_id: string; reason: string; frozen_at: string; }
interface FeatureConfig { menus: Record<string, boolean>; themes: Record<string, boolean>; modes: Record<string, boolean>; }
interface Role { role_id: string; role_name: string; is_default: boolean; features: FeatureConfig; }

const onlineClients = ref<ClientInfo[]>([]);
const allUsers = ref<UserRecord[]>([]);
const banMap = ref<Record<string, BanInfo>>({});
const freezeMap = ref<Record<string, FreezeInfo>>({});
const allRoles = ref<Role[]>([]);
const selectedRole = ref<Role | null>(null);

let pollTimer: ReturnType<typeof setInterval> | null = null;

const fetchClients = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/clients`, 'GET');
    onlineClients.value = data.clients || [];
  } catch { /* ignore */ }
};
const fetchUsers = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/users`, 'GET');
    allUsers.value = data.users || [];
    banMap.value = data.bans || {};
    freezeMap.value = data.frozen || {};
  } catch { /* ignore */ }
};
const fetchRoles = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles`, 'GET');
    allRoles.value = data.roles || [];
  } catch { /* ignore */ }
};
const startPolling = () => {
  fetchClients(); fetchUsers(); fetchRoles();
  pollTimer = setInterval(() => { fetchClients(); fetchUsers(); fetchRoles(); }, 5000);
};
const stopPolling = () => { if (pollTimer) { clearInterval(pollTimer); pollTimer = null; } };

// === User Actions ===
const selectedUserId = ref<string | null>(null);
const selectUser = (uid: string) => { selectedUserId.value = uid; activeTab.value = 'users'; };

const adminPost = async (endpoint: string, body: any) => {
  try {
    const data = await VrcApi.request(`${serverUrl.value}${endpoint}`, 'POST', body);
    
    if (data && data.success === false) {
       throw new Error(data.message || '操作未成功');
    }
    
    addLog(`[INFO] ${data?.message || 'OK'}`);
    showToast(data?.message || '操作成功');
    fetchClients(); 
    fetchUsers();
  } catch (e: any) { 
    addLog(`[ERROR] ${e.message || e}`); 
    showToast(`操作失败: ${e.message || e}`);
  }
};

const kickUser = (uid: string) => adminPost('/api/admin/kick', { user_id: uid });
const unbanUser = (uid: string) => adminPost('/api/admin/unban', { user_id: uid });
const unfreezeUser = (uid: string) => adminPost('/api/admin/unfreeze', { user_id: uid });
const removeUser = (uid: string) => { if (confirm(t('role.confirm_remove'))) adminPost('/api/admin/remove', { user_id: uid }); };

// Ban Dialog
const showBanDialog = ref(false);
const dialogUser = ref<UserRecord | null>(null);
const banReason = ref('');
const banDuration = ref<number | undefined>(undefined);
const openBanDialog = (u: UserRecord) => { dialogUser.value = u; banReason.value = ''; banDuration.value = undefined; showBanDialog.value = true; };
const confirmBan = () => {
  if (!dialogUser.value) return;
  if (!banReason.value.trim()) {
    alert(t('role.please_input_ban_reason'));
    return;
  }
  adminPost('/api/admin/ban', { user_id: dialogUser.value.user_id, reason: banReason.value.trim(), duration_hours: banDuration.value || null });
  showBanDialog.value = false;
};

// Freeze Dialog
const showFreezeDialog = ref(false);
const freezeReason = ref('');
const openFreezeDialog = (u: UserRecord) => { dialogUser.value = u; freezeReason.value = ''; showFreezeDialog.value = true; };
const confirmFreeze = () => {
  if (!dialogUser.value) return;
  if (!freezeReason.value.trim()) {
    alert(t('role.please_input_freeze_reason'));
    return;
  }
  adminPost('/api/admin/freeze', { user_id: dialogUser.value.user_id, reason: freezeReason.value.trim() });
  showFreezeDialog.value = false;
};

// Roles Management
const createNewRole = () => {
  const newRole: Role = {
    role_id: 'role_' + Date.now(),
    role_name: t('role.new_role'),
    is_default: false,
    features: {
      menus: { "dashboard":true, "feed":true, "friendlog":true, "locations":true, "charts":true, "playerlist":true, "gallery":true, "social":true, "search":true, "notifications":true, "groups":true, "avatars":true, "favorites":true, "moderation":true, "heatmap":true, "gamelog":true, "notes":true, "presets":true, "tools":true, "translator":true, "ovr":true, "env":true, "export":true, "settings":true },
      themes: { "dog":true, "cat":true, "helmet":true, "mono":true },
      modes: { "pc":true, "vr":true }
    }
  };
  selectedRole.value = newRole;
};

const selectRole = (r: Role) => { selectedRole.value = JSON.parse(JSON.stringify(r)); };

const saveRole = async () => {
  if (!selectedRole.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles`, 'POST', selectedRole.value);
    if(data.success) {
      alert(t('settings.saved'));
      fetchRoles();
    }
  } catch (e: any) { 
    addLog(`[ERROR] 保存失败: ${e.message || e}`);
    showToast(`保存失败: ${e.message || e}`);
  }
};

const deleteRole = async (role_id: string) => {
  if(!confirm('确定删除此角色吗？')) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles/delete`, 'POST', { role_id });
    if(data.success) {
       if(selectedRole.value?.role_id === role_id) selectedRole.value = null;
       showToast('删除成功');
       fetchRoles(); fetchUsers();
    } else {
       addLog(`[WARN] ${data.message}`);
       alert(data.message);
    }
  } catch {}
};

const setDefaultRole = async (role_id: string) => {
  try {
    await VrcApi.request(`${serverUrl.value}/api/admin/roles/set_default`, 'POST', { role_id });
    showToast('默认角色已更改');
    fetchRoles();
  } catch {}
};

const setUserRole = async (user_id: string, role_id: string | null) => {
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/users/set_role`, 'POST', { user_id, role_id });
    if (data.success) {
      showToast('角色分配成功！');
    }
  } catch {}
};

// === Server Lifecycle ===
const startLocalServer = async () => {
  try {
    addLog('[INFO] 正在启动服务端...');
    await SysApi.startServer({ host: serverHost.value, port: serverPort.value });
    isRunning.value = true;
    setTimeout(startPolling, 500);
  } catch (err: any) { addLog('[ERROR] 启动失败: ' + (err.message || err)); }
};
const stopLocalServer = async () => {
  try {
    addLog('[INFO] 正在停止服务端...');
    await SysApi.stopServer();
    isRunning.value = false;
    stopPolling();
    addLog('[INFO] 服务端已成功停止');
  } catch (err: any) {
    addLog('[ERROR] 停止失败: ' + (err.message || err));
  }
};

// === Lifecycle ===
onMounted(async () => {
  addLog('[INFO] ' + t('role.dashboard_title') + ' 已加载。请配置后点击启动。');
  unlistenLog = await listen<string>('server_log', (e) => addLog(e.payload));
  unlistenClients = await listen<string>('clients_updated', () => { fetchClients(); fetchUsers(); });
  
  try {
    const running = await SysApi.isServerRunning();
    if (running) {
      isRunning.value = true;
      addLog('[INFO] 检测到服务端已经在后台运行中...');
      setTimeout(startPolling, 500);
    }
  } catch (err: any) {
    console.warn("检查服务端状态失败", err);
  }
});
onUnmounted(() => { unlistenLog?.(); unlistenClients?.(); stopPolling(); });

const openNewClient = async () => {
  try { addLog('[INFO] 正在启动新客户端窗口...'); await SysApi.openNewClient(); }
  catch (err: any) { addLog('[ERROR] 启动客户端失败: ' + (err.message || err)); }
};
const stopAndExit = () => { stopPolling(); emit('exit'); };
</script>

<style scoped>
@keyframes radar-sweep {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.animate-radar-sweep {
  animation: radar-sweep 4s linear infinite;
}
.animate-spin-slow {
  animation: spin 8s linear infinite;
}
.custom-scrollbar::-webkit-scrollbar {
  width: 6px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: rgba(15, 23, 42, 0.5);
  border-radius: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: rgba(6, 182, 212, 0.3);
  border-radius: 4px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: rgba(6, 182, 212, 0.6);
}
</style>
