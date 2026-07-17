<template>
  <div 
    class="h-screen w-screen flex flex-col bg-background text-text font-mono p-4 relative overflow-hidden transition-colors duration-500"
    :style="themeStyles"
  >
    <div class="blob blob-1"></div>
    <div class="blob blob-2"></div>
     <div class="absolute inset-0 bg-[var(--theme-bg-main)]/10 pointer-events-none" />

    <!-- Animated background grid -->
    <div class="absolute inset-0 bg-[url('data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iNjAiIGhlaWdodD0iNjAiIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+PGcgb3BhY2l0eT0iMC4wNSI+PHBhdGggZD0iTTAgNjBMMjAgNjBMMjAgMEwwIDBaIiBmaWxsPSIjMThBMEZCIi8+PHBhdGggZD0iTTYwIDBMMCAwTDAgMjBMNjAgMjBaIiBmaWxsPSIjMThBMEZCIi8+PC9nPjwvc3ZnPg==')] opacity-10 pointer-events-none" />

    <!-- Top Bar -->
    <div class="flex items-center justify-between mb-4 pb-4 border-border-soft shrink-0 relative z-50">
      <div class="flex items-center gap-4">
        <div class="relative">
          <div
            class="absolute inset-0 bg-primary blur-md opacity-20"
            :class="{'animate-pulse': isRunning}"
          />
          <div
            class="w-12 h-12 rounded-2xl flex items-center justify-center transition-all shadow-sm"
            :class="isRunning ? 'bg-primary border-primary shadow-primary/30' : 'bg-surface border-border-soft hover:bg-surface-hover'"
          >
            <Server
              class="w-6 h-6 transition-colors"
              :class="isRunning ? 'text-white' : 'text-text-muted'"
            />
          </div>
        </div>
        <div>
          <h1 class="text-2xl font-black tracking-widest uppercase bg-clip-text text-transparent bg-gradient-to-r from-primary to-primary-hover">
            {{ t('role.dashboard_title') }}
          </h1>
          <p
            class="text-xs tracking-widest uppercase flex items-center gap-2 mt-0.5"
            :class="isRunning ? 'text-primary' : 'text-text-muted'"
          >
            <span
              class="w-1.5 h-1.5 rounded-full"
              :class="isRunning ? 'bg-primary animate-ping' : 'bg-text-muted'"
            />
            {{
              isRunning
                ? (isRemoteMode
                  ? t('role.dashboard_remote_running', { url: serverUrl })
                  : t('role.dashboard_running', { port: serverPort }))
                : t('role.server_stopped')
            }}
          </p>
        </div>
      </div>
       <div class="flex gap-3 items-center">
         <!-- Theme Switcher -->
         <div class="relative group">
           <button class="flex items-center gap-2 px-4 py-2.5 bg-surface hover:bg-surface-hover border-border-soft rounded-2xl text-text hover:text-primary text-xs transition-all font-bold shadow-sm">
             <Palette class="w-4 h-4" />
             <span class="capitalize">{{ t(themes[currentThemeId]?.name || currentThemeId) }}</span>
           </button>
           <div class="absolute top-full right-0 mt-2 w-40 bg-surface backdrop-blur-2xl border-border-soft rounded-2xl shadow-xl opacity-0 invisible group-hover:opacity-100 group-hover:visible transition-all z-50 flex flex-col p-2 gap-1">
             <button
               v-for="themeItem in Object.values(themes)"
               :key="themeItem.id"
               class="w-full text-left px-3 py-2 text-xs transition-all rounded-xl flex items-center justify-between"
               :class="currentThemeId === themeItem.id ? 'bg-primary text-white font-bold shadow-md shadow-primary/30' : 'text-text-muted hover:bg-surface-hover hover:text-primary'"
               @click="setTheme(themeItem.id as ThemeId)"
             >
               {{ t(themeItem.name) }}
               <Check v-if="currentThemeId === themeItem.id" class="w-3 h-3" />
             </button>
           </div>
         </div>

        <button
          class="flex items-center gap-2 px-4 py-2.5 bg-surface hover:bg-surface-hover border-border-soft rounded-2xl text-text hover:text-primary text-xs font-bold transition-all shadow-sm"
          @click="cycleLanguage"
        >
          <Globe class="w-4 h-4" /> {{ currentLangLabel }}
        </button>
        <button v-if="!isRemoteMode" class="flex items-center gap-2 px-4 py-2 bg-primary hover:bg-primary-hover text-white rounded-xl shadow-md shadow-primary/30 text-xs font-bold transition-all" @click="openNewClient">
          <Monitor class="w-4 h-4" /> {{ t('role.dashboard_open_client') }}
        </button>
        <button class="flex items-center gap-2 px-4 py-2 bg-surface hover:bg-surface-hover text-text hover:text-red-500 border border-border-soft rounded-xl shadow-sm text-xs font-bold transition-all" @click="stopAndExit">
          <LogOut class="w-4 h-4" /> {{ t('role.back') }}
        </button>
      </div>
    </div>

    <!-- Main Area -->
    <div class="flex-1 flex gap-4 overflow-hidden relative z-10">
      <!-- Left Panel: Server Config + Radar Map -->
      <div class="w-80 flex flex-col gap-4 shrink-0 overflow-hidden">
        <!-- Connection Info / Start -->
        <div class="glass-panel p-5">
          <h2 class="text-[10px] text-primary mb-4 uppercase tracking-[0.2em] font-bold flex items-center gap-2">
            <Activity class="w-3 h-3" />
            {{ t('role.dashboard_conn_info') }}
          </h2>
          <div class="grid grid-cols-2 gap-1 p-1 mb-4 bg-surface-hover border border-border-soft rounded-lg">
            <button
              class="min-h-9 rounded-md text-xs font-bold flex items-center justify-center gap-2 transition-colors"
              :class="!isRemoteMode ? 'bg-primary text-white' : 'text-text-muted hover:text-primary'"
              @click="selectServerMode('local')"
            >
              <Monitor class="w-4 h-4" />
              {{ t('role.local_service') }}
            </button>
            <button
              class="min-h-9 rounded-md text-xs font-bold flex items-center justify-center gap-2 transition-colors"
              :class="isRemoteMode ? 'bg-primary text-white' : 'text-text-muted hover:text-primary'"
              @click="selectServerMode('remote')"
            >
              <Cloud class="w-4 h-4" />
              {{ t('role.remote_service') }}
            </button>
          </div>
          <div
            v-if="isRemoteMode"
            class="space-y-3"
          >
            <div>
              <label class="text-[10px] text-text block mb-1 uppercase tracking-wider">{{ t('role.remote_server_address') }}</label>
              <input
                v-model="remoteServerUrl"
                type="url"
                class="w-full bg-surface/50 border border-border-soft rounded-lg px-3 py-2 text-xs outline-none text-text-strong font-mono transition-colors"
                :placeholder="t('role.server_address_ph')"
                @keydown.enter="connectRemoteServer"
              >
            </div>
            <div
              v-if="isRunning"
              class="text-xs text-primary p-3 rounded-lg border border-primary/20 bg-primary/5 break-all"
            >
              {{ serverUrl }}
            </div>
            <button
              class="w-full mt-4 py-3 font-bold rounded-xl text-xs flex justify-center items-center gap-2 shadow-md transition-all"
              :class="isRunning ? 'bg-surface-hover text-primary border border-primary/20' : 'bg-primary hover:bg-primary-hover text-white shadow-primary/30'"
              @click="isRunning ? disconnectRemoteServer() : connectRemoteServer()"
            >
              <RefreshCcw v-if="isRunning" class="w-4 h-4" />
              <Cloud v-else class="w-4 h-4" />
              {{ isRunning ? t('role.remote_reconnect') : t('role.connect_server') }}
            </button>
          </div>
          <div
            v-else-if="!isRunning"
            class="space-y-3"
          >
            <div>
              <label class="text-[10px] text-text block mb-1 uppercase tracking-wider">BIND IP</label>
              <input
                v-model="serverHost"
                class="w-full bg-surface/50 border border-border-soft rounded-lg px-3 py-2 text-xs outline-none text-text-strong font-mono transition-colors"
              >
            </div>
            <div>
              <label class="text-[10px] text-text-soft block mb-1 uppercase tracking-wider">PORT</label>
              <input
                v-model.number="serverPort"
                type="number"
                class="w-full bg-surface/50 border border-border-soft rounded-lg px-3 py-2 text-xs outline-none text-text-strong font-mono transition-colors"
              >
            </div>
            <button
              class="w-full mt-4 py-3 bg-primary hover:bg-primary-hover text-white font-bold rounded-xl text-xs flex justify-center items-center gap-2 shadow-md shadow-primary/30 transition-all"
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
            <div class="text-2xl font-black tracking-widest bg-surface-hover/40 backdrop-blur-md text-text p-3 rounded-lg border-border-soft text-center shadow-inner">
              <span class="text-primary">{{ serverHost }}</span><span class="text-primary/50">:</span><span class="text-primary">{{ serverPort }}</span>
            </div>
            <p class="text-xs text-primary text-center animate-pulse uppercase tracking-[0.2em]">
              {{ t('role.dashboard_listening') }}
            </p>
            <button
              class="w-full py-2.5 bg-red-500/10 text-red-500 hover:bg-red-500/20 font-bold rounded-lg text-xs border border-red-500/20 hover:border-red-500/40 flex justify-center items-center gap-2 transition-all"
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
        <div class="glass-panel rounded-2xl p-5 border-primary/30 flex-1 overflow-hidden flex flex-col relative group">
          <div class="absolute inset-0 bg-[radial-gradient(circle_at_center,_var(--tw-gradient-stops))] from-primary/10 to-transparent pointer-events-none" />
          
          <div class="flex justify-between items-center mb-4 relative z-10">
            <h2 class="text-[10px] text-primary uppercase tracking-[0.2em] font-bold flex items-center gap-2">
              <Radar
                class="w-3 h-3"
                :class="{'animate-spin-slow': isRunning}"
              />
              PLAYER RADAR
            </h2>
            <span class="text-xs font-black text-primary bg-primary/20/50 px-2 py-0.5 rounded border-primary/30">{{ onlineClients.length }} ONLINE</span>
          </div>

          <div
            v-if="onlineClients.length > 0"
            class="flex-1 overflow-y-auto space-y-2 relative z-10 custom-scrollbar pr-1"
          >
            <div
              v-for="c in onlineClients"
              :key="c.user_id"
              class="flex items-center gap-3 p-3 bg-surface rounded-xl border-border-soft hover:border-primary/50 hover:bg-primary/20/30 cursor-pointer transition-all glass-panel-hover"
              @click="selectUser(c.user_id)"
            >
              <div class="relative">
                <div class="absolute inset-0 bg-primary blur-sm opacity-50 animate-pulse" />
                <img
                  :src="c.avatar_url"
                  class="w-10 h-10 rounded-full border-2 border-primary/30 object-cover relative z-10"
                  @error="(e) => (e.target as HTMLImageElement).src='https://assets.vrchat.com/www/images/default_avatar.png'"
                >
              </div>
              <div class="min-w-0 flex-1">
                <div class="text-sm font-bold text-text truncate drop-shadow-md">
                  {{ c.display_name }}
                </div>
                <div class="text-[10px] text-text-muted truncate font-mono mt-0.5">
                  {{ c.ip_address }}
                </div>
              </div>
              <div class="text-primary/50 hover:text-primary">
                <ChevronRight class="w-4 h-4" />
              </div>
            </div>
          </div>
          <div
            v-else
            class="flex-1 flex flex-col items-center justify-center text-primary/50 text-xs text-center relative z-10 gap-3"
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
              class="w-[200%] h-[200%] absolute top-[-50%] left-[-50%] border-primary/30 rounded-full animate-radar-sweep"
              style="background: conic-gradient(from 0deg, transparent 0deg, transparent 90deg, rgba(6, 182, 212, 0.1) 180deg);"
            />
          </div>
        </div>
      </div>

      <!-- Right Panel: Tabs -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Tab Buttons -->
        <div class="flex gap-2 mb-4 shrink-0 glass-panel p-1.5 rounded-xl border-primary/30 inline-flex w-fit shadow-md">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            class="px-5 py-2 rounded-lg text-xs font-bold transition-all uppercase tracking-wider relative overflow-hidden"
            :class="activeTab === tab.key ? 'text-white' : 'text-text-muted hover:text-primary'"
            @click="activeTab = tab.key"
          >
            <div
              v-if="activeTab === tab.key"
              class="absolute inset-0 bg-primary -z-10"
            />
            {{ tab.label }}
          </button>
        </div>

        <div
          v-show="activeTab === 'logs'"
          class="flex-1 terminal-black rounded-2xl border border-zinc-800/50 p-4 flex flex-col overflow-hidden relative shadow-inner"
          style="background: rgba(10, 10, 10, 0.95) !important;"
        >
          <div class="flex justify-between items-center mb-3">
            <span class="text-[10px] text-zinc-400 uppercase tracking-widest font-bold flex items-center gap-2"><Terminal class="w-3 h-3" /> SYSTEM LOGS</span>
            <button
              class="text-[10px] text-zinc-400 hover:text-white uppercase tracking-wider border-zinc-800 hover:border-zinc-700 px-2 py-1 rounded transition-colors"
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
              class="mb-1 leading-relaxed break-words whitespace-pre-wrap flex gap-3 hover:bg-white/5 px-2 py-0.5 rounded transition-colors"
            >
              <span class="text-zinc-500 shrink-0 select-none">[{{ log.time }}]</span>
              <span class="shrink-0 select-none font-bold" :class="{'text-blue-400': log.level==='INFO','text-red-400': log.level==='ERROR','text-yellow-400': log.level==='WARN','text-emerald-400': log.level==='SUCCESS'}">[{{ log.level }}]</span>
              <span class="text-zinc-300">{{ log.content }}</span>
            </div>
            <div
              v-if="logs.length===0"
              class="text-zinc-600 mt-10 text-center uppercase tracking-widest opacity-50 flex flex-col items-center gap-2 m-auto"
            >
              <Terminal class="w-8 h-8" /> {{ t('debug.waiting_api') || 'AWAITING INPUT...' }}
            </div>
          </div>
        </div>

        <!-- Tab: User Management -->
        <div
          v-show="activeTab === 'users'"
          class="flex-1 glass-panel rounded-2xl border-primary/30 p-1 flex flex-col overflow-hidden"
        >
          <div class="flex-1 overflow-y-auto custom-scrollbar">
            <table v-if="allUsers.length > 0" class="w-full text-left border-collapse">
              <thead>
                <tr class="text-[10px] text-primary uppercase tracking-[0.2em] border-b border-border-soft">
                  <th class="p-4 font-bold">{{ t('role.user') || 'USER' }}</th>
                  <th class="p-4 font-bold">{{ t('role.status') || 'STATUS' }}</th>
                  <th class="p-4 font-bold">{{ t('role.role') || 'ROLE' }}</th>
                  <th class="p-4 font-bold">{{ t('role.actions') || 'ACTIONS' }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="u in allUsers" :key="u.user_id" class="border-b border-border-soft/30 hover:bg-surface/40 transition-colors group">
                  <td class="p-4">
                    <div class="flex items-center gap-3">
                      <div class="relative">
                        <img :src="u.avatar_url" class="w-8 h-8 rounded-full border border-border-soft object-cover" @error="(e) => (e.target as HTMLImageElement).src='https://assets.vrchat.com/www/images/default_avatar.png'">
                        <div v-if="u.is_online" class="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 bg-emerald-500 border-2 border-surface rounded-full" />
                      </div>
                      <div class="min-w-0">
                        <div class="text-xs font-bold text-text truncate">{{ u.display_name }}</div>
                        <div class="text-[10px] text-text-muted truncate font-mono">{{ u.user_id }}</div>
                      </div>
                    </div>
                  </td>
                  <td class="p-4">
                    <div class="flex flex-col gap-1">
                       <span :class="u.is_online ? 'text-emerald-400' : 'text-text-muted'" class="text-[10px] font-bold uppercase tracking-wider">
                         {{ u.is_online ? t('status.online') : t('status.offline') }}
                       </span>
                       <span v-if="banMap[u.user_id]" class="text-[9px] text-red-500 font-bold uppercase">{{ t('role.banned') }}</span>
                       <span v-if="freezeMap[u.user_id]" class="text-[9px] text-blue-400 font-bold uppercase">{{ t('role.frozen') }}</span>
                    </div>
                  </td>
                  <td class="p-4">
                    <CustomSelect 
                      :modelValue="u.role_id" 
                      @update:modelValue="(val) => setUserRole(u.user_id, val as string | null)"
                      :options="allRoles.map(role => ({ label: role.role_name, value: role.role_id }))"
                      class="w-36"
                    />
                  </td>
                  <td class="p-4">
                    <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                       <button v-if="u.is_online" class="p-1.5 hover:bg-yellow-500/20 text-yellow-500 rounded-lg transition-colors" :title="t('role.kick')" @click="kickUser(u.user_id)">
                         <LogOut class="w-4 h-4" />
                       </button>
                       <button v-if="!banMap[u.user_id]" class="p-1.5 hover:bg-red-500/20 text-red-500 rounded-lg transition-colors" :title="t('role.ban')" @click="openBanDialog(u)">
                         <ShieldAlert class="w-4 h-4" />
                       </button>
                       <button v-else class="p-1.5 hover:bg-emerald-500/20 text-emerald-500 rounded-lg transition-colors" :title="t('role.unban')" @click="unbanUser(u.user_id)">
                         <UserCheck class="w-4 h-4" />
                       </button>
                       <button v-if="!freezeMap[u.user_id]" class="p-1.5 hover:bg-blue-500/20 text-blue-500 rounded-lg transition-colors" :title="t('role.freeze')" @click="openFreezeDialog(u)">
                         <Snowflake class="w-4 h-4" />
                       </button>
                       <button v-else class="p-1.5 hover:bg-emerald-500/20 text-emerald-500 rounded-lg transition-colors" :title="t('role.unfreeze')" @click="unfreezeUser(u.user_id)">
                         <Activity class="w-4 h-4" />
                       </button>
                       <button class="p-1.5 hover:bg-red-500/20 text-red-500 rounded-lg transition-colors" :title="t('role.remove')" @click="removeUser(u.user_id)">
                         <Trash2 class="w-4 h-4" />
                       </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
            <div
              v-else
              class="flex-1 flex flex-col items-center justify-center text-text-muted text-xs h-full gap-3 uppercase tracking-widest"
            >
              <Users class="w-10 h-10 opacity-30" />
              {{ t('role.no_user_records') }}
            </div>
          </div>
        </div>

        <!-- Tab: Roles -->
        <div
          v-show="activeTab === 'features'"
          class="flex-1 glass-panel rounded-2xl border-primary/30 p-1 flex overflow-hidden"
        >
          <!-- Roles List -->
          <div class="w-1/3 flex flex-col border-primary/30 p-3 bg-surface/30">
            <div class="flex justify-between items-center mb-4 shrink-0">
              <h3 class="text-[10px] font-bold text-primary uppercase tracking-widest flex items-center gap-2">
                <Shield class="w-3 h-3" /> {{ t('role.dashboard_roles_list') || 'ROLES LIST' }}
              </h3>
              <button
                class="px-2 py-1 bg-primary text-white hover:bg-primary-hover border border-primary rounded text-[10px] font-bold transition-colors shadow-sm shadow-primary/20"
                @click="createNewRole"
              >
                +
              </button>
            </div>
            <div class="flex-1 overflow-y-auto space-y-2 custom-scrollbar pr-1">
              <div
                v-for="r in allRoles"
                :key="r.role_id"
                class="p-3 rounded-xl cursor-pointer flex justify-between items-center transition-all group"
                :class="selectedRole?.role_id === r.role_id ? 'bg-primary/20 border-primary/50 shadow-[0_0_15px_rgba(6,182,212,0.1)]' : 'bg-surface/80 border-border-soft text-text hover:border-border-soft'"
                @click="selectRole(r)"
              >
                <div
                  class="text-xs font-bold"
                  :class="selectedRole?.role_id === r.role_id ? 'text-primary' : 'text-text'"
                >
                  {{ r.role_name }} 
                  <span
                    v-if="r.is_default"
                    class="text-[8px] uppercase tracking-wider bg-surface-hover text-text px-1.5 py-0.5 rounded ml-2 font-mono"
                  >{{ t('role.default') }}</span>
                </div>
                <div
                  v-if="selectedRole?.role_id === r.role_id"
                  class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity"
                >
                  <button
                    v-if="!r.is_default"
                    class="text-text hover:text-primary transition-colors"
                    :title="t('role.set_default')"
                    @click.stop="setDefaultRole(r.role_id)"
                  >
                    <Star class="w-3.5 h-3.5" />
                  </button>
                  <button
                    v-if="!r.is_default"
                    class="text-text hover:text-red-400 transition-colors"
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
            <div class="mb-5 shrink-0 bg-surface/80 p-4 rounded-xl border-primary/30">
              <label class="text-[10px] text-primary font-bold block mb-2 uppercase tracking-widest">{{ t('role.role_name') }}</label>
              <input
                v-model="selectedRole.role_name"
                class="w-full bg-surface/50 border border-border-soft rounded-lg px-3 py-2 text-sm outline-none text-text-strong font-bold transition-colors shadow-inner"
              >
            </div>
            
            <div class="space-y-6 flex-1">
              <div>
                <h3 class="text-[10px] font-bold text-text mb-3 uppercase tracking-widest border-border-soft pb-1">
                  {{ t('role.feature_menus') }}
                </h3>
                <div class="grid grid-cols-3 gap-2">
                  <label
                    v-for="(enabled, key) in selectedRole.features.menus"
                    :key="key"
                    class="flex items-center gap-3 p-2 bg-surface/80 rounded-lg border-border-soft hover:border-border-soft cursor-pointer text-xs group transition-colors"
                    :class="{'bg-primary/20/30 border-primary/30': enabled}"
                  >
                    <div
                      class="w-4 h-4 rounded flex items-center justify-center transition-colors"
                      :class="enabled ? 'bg-primary border-primary/30' : 'bg-surface border-border-soft'"
                    >
                      <Check
                        v-if="enabled"
                        class="w-3 h-3 text-white font-bold"
                        stroke-width="3"
                      />
                    </div>
                    <input
                      v-model="selectedRole.features.menus[key]"
                      type="checkbox"
                      class="hidden"
                    >
                    <span class="text-text font-medium group-hover:text-text transition-colors">{{ t('sidebar.' + key) || key }}</span>
                  </label>
                </div>
              </div>
              
              <div>
                <h3 class="text-[10px] font-bold text-text mb-3 uppercase tracking-widest border-border-soft pb-1">
                  {{ t('role.feature_modes') }}
                </h3>
                <div class="grid grid-cols-2 gap-2">
                  <label
                    v-for="(enabled, key) in selectedRole.features.modes"
                    :key="key"
                    class="flex items-center gap-3 p-2 bg-surface/80 rounded-lg border-border-soft hover:border-border-soft cursor-pointer text-xs group transition-colors"
                    :class="{'bg-primary/20/30 border-primary/30': enabled}"
                  >
                    <div
                      class="w-4 h-4 rounded flex items-center justify-center transition-colors"
                      :class="enabled ? 'bg-primary border-primary/30' : 'bg-surface border-border-soft'"
                    >
                      <Check
                        v-if="enabled"
                        class="w-3 h-3 text-white font-bold"
                        stroke-width="3"
                      />
                    </div>
                    <input
                      v-model="selectedRole.features.modes[key]"
                      type="checkbox"
                      class="hidden"
                    >
                    <span class="text-text font-medium group-hover:text-text transition-colors uppercase tracking-wider">{{ t('role.mode_' + key) || (key === 'pc' ? 'PC Desktop' : key === 'vr' ? 'VR Overlay' : key) }}</span>
                  </label>
                </div>
              </div>
              
              <div>
                <h3 class="text-[10px] font-bold text-text mb-3 uppercase tracking-widest border-border-soft pb-1">
                  {{ t('role.feature_themes') }}
                </h3>
                <div class="grid grid-cols-3 gap-2">
                  <label
                    v-for="(enabled, key) in selectedRole.features.themes"
                    :key="key"
                    class="flex items-center gap-3 p-2 bg-surface/80 rounded-lg border-border-soft hover:border-border-soft cursor-pointer text-xs group transition-colors"
                    :class="{'bg-primary/20/30 border-primary/30': enabled}"
                  >
                    <div
                      class="w-4 h-4 rounded flex items-center justify-center transition-colors"
                      :class="enabled ? 'bg-primary border-primary/30' : 'bg-surface border-border-soft'"
                    >
                      <Check
                        v-if="enabled"
                        class="w-3 h-3 text-white font-bold"
                        stroke-width="3"
                      />
                    </div>
                    <input
                      v-model="selectedRole.features.themes[key]"
                      type="checkbox"
                      class="hidden"
                    >
                    <span class="text-text font-medium group-hover:text-text transition-colors uppercase tracking-wider">{{ t('role.theme_' + key) || (key.charAt(0).toUpperCase() + key.slice(1)) }}</span>
                  </label>
                </div>
              </div>
            </div>
            
            <div class="mt-6 pt-4 border-primary/30 shrink-0">
              <button
                class="w-full py-2.5 bg-gradient-to-r from-primary/80 to-primary/80 hover:from-primary/80 hover:to-primary/80 text-white font-black rounded-xl text-xs uppercase tracking-widest shadow-md shadow-primary/30 transition-all flex items-center justify-center gap-2"
                @click="saveRole"
              >
                <Save class="w-4 h-4" /> {{ t('role.save_role') }}
              </button>
            </div>
          </div>
          <div
            v-else
            class="flex-1 flex flex-col items-center justify-center text-text-muted text-xs gap-3 uppercase tracking-widest font-bold"
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
      class="fixed inset-0 bg-surface/80 backdrop-blur-md flex items-center justify-center z-[999]"
      @click.self="showBanDialog=false"
    >
      <div class="bg-surface/80 rounded-2xl p-6 w-96 border-red-900/50 shadow-[0_0_30px_rgba(220,38,38,0.2)]">
        <h3 class="text-sm font-black mb-4 text-red-500 uppercase tracking-widest flex items-center gap-2">
          <ShieldAlert class="w-4 h-4" /> {{ t('role.ban_user') }}
        </h3>
        <p class="text-text font-bold mb-4 bg-surface p-2 rounded-lg border-border-soft">
          {{ dialogUser?.display_name }}
        </p>
        <div class="space-y-4">
          <div>
            <label class="text-[10px] text-text block mb-1 uppercase tracking-wider">{{ t('role.ban_reason') }}</label>
            <input
              v-model="banReason"
              class="w-full bg-surface border-red-900/30 rounded-lg px-3 py-2 text-sm outline-none focus:border-red-500 text-text"
              :placeholder="t('role.ban_reason_ph')"
            >
          </div>
          <div>
            <label class="text-[10px] text-text block mb-1 uppercase tracking-wider">{{ t('role.ban_duration') }}</label>
            <input
              v-model.number="banDuration"
              type="number"
              class="w-full bg-surface border-red-900/30 rounded-lg px-3 py-2 text-sm outline-none focus:border-red-500 text-text"
              :placeholder="t('role.ban_duration_ph')"
            >
          </div>
          <div class="flex gap-3 justify-end mt-6 pt-4 border-border-soft">
            <button
              class="px-4 py-2 bg-surface-hover hover:bg-surface-active rounded-lg text-xs font-bold text-text uppercase tracking-wider transition-colors"
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
       class="fixed inset-0 bg-[var(--theme-bg-main)]/50 backdrop-blur-sm flex items-center justify-center z-[999]"
       @click.self="showFreezeDialog=false"
     >
      <div class="bg-surface rounded-2xl p-6 w-96 border border-border-soft shadow-xl">
        <h3 class="text-sm font-black mb-4 text-primary uppercase tracking-widest flex items-center gap-2">
          <Snowflake class="w-4 h-4" /> {{ t('role.freeze_user') }}
        </h3>
        <p class="text-text font-bold mb-4 bg-surface-hover p-2 rounded-lg border border-border-soft">
          {{ dialogUser?.display_name }}
        </p>
        <div class="space-y-4">
          <div>
            <label class="text-[10px] text-text-muted block mb-1 uppercase tracking-wider">{{ t('role.freeze_reason') }}</label>
            <input
              v-model="freezeReason"
              class="w-full bg-surface border border-border-soft focus:border-primary rounded-lg px-3 py-2 text-sm outline-none text-text transition-colors"
              :placeholder="t('role.freeze_reason_ph')"
            >
          </div>
          <div class="flex gap-3 justify-end mt-6 pt-4 border-t border-border-soft">
            <button
              class="px-4 py-2 bg-surface hover:bg-surface-hover rounded-lg text-xs font-bold text-text-muted border border-border-soft uppercase tracking-wider transition-colors"
              @click="showFreezeDialog=false"
            >
              {{ t('role.cancel') }}
            </button>
            <button
              class="px-4 py-2 bg-primary hover:bg-primary-hover text-white font-bold rounded-lg text-xs uppercase tracking-widest transition-colors"
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
      class="fixed top-8 left-1/2 transform -translate-x-1/2 px-6 py-3 bg-primary/20 backdrop-blur-md border-primary/30 text-cyan-50 rounded-xl shadow-md shadow-primary/50 z-[1000] text-sm font-black tracking-widest transition-all uppercase flex items-center gap-2"
    >
      <div class="w-2 h-2 bg-primary rounded-full animate-pulse" />
      {{ toastMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import CustomSelect from './CustomSelect.vue';
import { useToast } from "../composables/useToast";

const toast = useToast();
import { ref, onMounted, onUnmounted, nextTick, computed, reactive } from 'vue';
import { Server, Globe, Monitor, LogOut, Play, Square, Activity, Radar, ChevronRight, Terminal, Users, Shield, Star, Trash2, ShieldAlert, Snowflake, Check, Save, Palette, UserCheck, Cloud, RefreshCcw } from 'lucide-vue-next';
import { currentThemeId, setTheme, themes, type ThemeId } from '../theme';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { isTauri } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { SysApi, DbApi, VrcApi } from '../api';
import { useStorage } from '@vueuse/core';
import { getLocaleLabel, getNextLocale, setAppLocale } from '../i18n';

// 动态注入 CSS 变量
const themeStyles = computed(() => {
  const colors = themes[currentThemeId.value]?.colors || themes.dog.colors;
  return {
    '--theme-bg-main': colors.bgMain,
    '--theme-surface': colors.surface,
    '--theme-surface-hover': colors.surfaceHover,
    '--theme-blob1': colors.blob1,
    '--theme-blob2': colors.blob2,
    '--theme-border-soft': colors.borderSoft,
    '--theme-border-strong': colors.borderStrong,
    '--theme-text-strong': colors.textStrong,
    '--theme-text-soft': colors.textSoft,
    '--theme-active-bg': colors.activeBg,
    '--theme-primary-btn-bg': colors.primaryBtnBg,
    '--theme-primary-btn-hover': colors.primaryBtnHover,
    '--theme-terminal-bg': colors.terminalBg,
    '--theme-glass-effect': colors.glassEffect,
    '--theme-primary': colors.primaryBtnBg,
    '--theme-primary-hover': colors.primaryBtnHover,
  };
});

const { t, locale } = useI18n();
const props = withDefaults(defineProps<{
  initialMode?: 'local' | 'remote';
  remoteUrl?: string;
  adminPassword: string;
}>(), {
  initialMode: 'local',
  remoteUrl: '',
});
const currentLangLabel = computed(() => getLocaleLabel(locale.value));
const cycleLanguage = () => {
  const nextLang = setAppLocale(getNextLocale(locale.value), { notify: true });
  locale.value = nextLang;
  DbApi.saveSetting({ key: 'language', value: JSON.stringify(nextLang) }).catch(() => {});
};

const emit = defineEmits(['exit']);
const serverHost = useStorage('vrc_dashboard_host', '0.0.0.0');
const serverPort = useStorage('vrc_dashboard_port', 11451);
const serverMode = ref<'local' | 'remote'>(props.initialMode);
const remoteServerUrl = useStorage(
  'vrc_dashboard_remote_url',
  props.remoteUrl || 'http://127.0.0.1:11451',
);
const isRunning = ref(false);
const isRemoteMode = computed(() => serverMode.value === 'remote');
const activeTab = ref('logs');
const tabs = computed(() => [
  { key: 'logs', label: t('role.dashboard_logs_tab') },
  { key: 'users', label: t('role.dashboard_users_tab') },
  { key: 'features', label: t('role.dashboard_roles_tab') },
]);

const toastMessage = ref('');
const showToast = (msg: string) => {
  toastMessage.value = msg;
  setTimeout(() => toastMessage.value = '', 3000);
};

// === Server URL helper ===
const normalizeServerUrl = (value: string) => {
  let normalized = value.trim();
  if (!normalized) return '';
  if (!/^https?:\/\//i.test(normalized)) normalized = `http://${normalized}`;
  return normalized.replace('0.0.0.0', '127.0.0.1').replace(/\/+$/, '');
};

const serverUrl = computed(() => {
  if (isRemoteMode.value) return normalizeServerUrl(remoteServerUrl.value);
  const h = serverHost.value === '0.0.0.0' ? '127.0.0.1' : serverHost.value;
  return `http://${h}:${serverPort.value}`;
});
const adminHeaders = computed(() => ({
  'x-vrcdog-admin-password': props.adminPassword,
}));

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
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/clients`, {
      method: 'GET',
      headers: adminHeaders.value,
    });
    onlineClients.value = data.clients || [];
  } catch { /* ignore */ }
};
const fetchUsers = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/users`, {
      method: 'GET',
      headers: adminHeaders.value,
    });
    allUsers.value = data.users || [];
    banMap.value = data.bans || {};
    freezeMap.value = data.frozen || {};
  } catch { /* ignore */ }
};
const fetchRoles = async () => {
  if (!isRunning.value) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles`, {
      method: 'GET',
      headers: adminHeaders.value,
    });
    allRoles.value = data.roles || [];
  } catch { /* ignore */ }
};
const startPolling = () => {
  stopPolling();
  fetchClients(); fetchUsers(); fetchRoles();
  pollTimer = setInterval(() => { fetchClients(); fetchUsers(); fetchRoles(); }, 5000);
};
const stopPolling = () => { if (pollTimer) { clearInterval(pollTimer); pollTimer = null; } };

// === User Actions ===
const selectedUserId = ref<string | null>(null);
const selectUser = (uid: string) => { selectedUserId.value = uid; activeTab.value = 'users'; };

const adminPost = async (endpoint: string, body: any) => {
  try {
    const data = await VrcApi.request(`${serverUrl.value}${endpoint}`, {
      method: 'POST',
      params: body,
      headers: adminHeaders.value,
    });
    
    if (data && data.success === false) {
       throw new Error(data.message);
    }
    
    addLog(`[INFO] ${data?.message || 'OK'}`);
    showToast(data?.message);
    fetchClients(); 
    fetchUsers();
  } catch (e: any) { 
    addLog(`[ERROR] ${e.message || e}`); 
    showToast(t('server.op_failed', { error: e.message || e }));
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
    toast.info(t('role.please_input_ban_reason'));
    return;
  }
  adminPost('/api/admin/ban', { 
    user_id: dialogUser.value.user_id, 
    reason: banReason.value.trim(), 
    duration_hours: banDuration.value ?? null 
  });
  showBanDialog.value = false;
};

// Freeze Dialog
const showFreezeDialog = ref(false);
const freezeReason = ref('');
const openFreezeDialog = (u: UserRecord) => { dialogUser.value = u; freezeReason.value = ''; showFreezeDialog.value = true; };
const confirmFreeze = () => {
  if (!dialogUser.value) return;
  if (!freezeReason.value.trim()) {
    toast.info(t('role.please_input_freeze_reason'));
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
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles`, {
      method: 'POST',
      params: selectedRole.value,
      headers: adminHeaders.value,
    });
    if(data.success) {
      toast.info(t('settings.saved'));
      fetchRoles();
    }
  } catch (e: any) { 
    addLog(t('app.save_fail', { error: e.message || e }));
    showToast(t('app.save_fail', { error: e.message || e }));
  }
};

const deleteRole = async (role_id: string) => {
  if(!confirm(t('app.confirm_delete_role'))) return;
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/roles/delete`, {
      method: 'POST',
      params: { role_id },
      headers: adminHeaders.value,
    });
    if(data.success) {
       if(selectedRole.value?.role_id === role_id) selectedRole.value = null;
       showToast(t('role.delete_success') || 'Success');
       fetchRoles(); fetchUsers();
    } else {
       addLog(`[WARN] ${data.message}`);
       toast.info(data.message);
    }
  } catch {}
};

const setDefaultRole = async (role_id: string) => {
  try {
    await VrcApi.request(`${serverUrl.value}/api/admin/roles/set_default`, {
      method: 'POST',
      params: { role_id },
      headers: adminHeaders.value,
    });
    showToast(t('role.default_changed') || 'Changed');
    fetchRoles();
  } catch {}
};

const setUserRole = async (user_id: string, role_id: string | null) => {
  try {
    const data = await VrcApi.request(`${serverUrl.value}/api/admin/users/set_role`, {
      method: 'POST',
      params: { user_id, role_id },
      headers: adminHeaders.value,
    });
    if (data.success) {
      showToast(t('role.assign_success') || 'Success');
    }
  } catch {}
};

// === Server Lifecycle ===
const connectRemoteServer = async () => {
  const normalized = normalizeServerUrl(remoteServerUrl.value);
  if (!normalized) {
    addLog(`[ERROR] ${t('role.error_require_url')}`);
    return;
  }
  remoteServerUrl.value = normalized;
  try {
    await SysApi.pingServer({ url: normalized });
    await VrcApi.request(`${normalized}/api/admin/auth`, {
      method: 'POST',
      params: { password: props.adminPassword },
    });
    isRunning.value = true;
    addLog(`[SUCCESS] ${t('role.remote_connected', { url: normalized })}`);
    startPolling();
  } catch (error: any) {
    isRunning.value = false;
    stopPolling();
    addLog(`[ERROR] ${t('role.connection_failed')}${error?.message || error}`);
  }
};

const disconnectRemoteServer = async () => {
  isRunning.value = false;
  stopPolling();
  onlineClients.value = [];
  allUsers.value = [];
  allRoles.value = [];
  await connectRemoteServer();
};

const selectServerMode = async (mode: 'local' | 'remote') => {
  if (serverMode.value === mode) return;
  stopPolling();
  isRunning.value = false;
  serverMode.value = mode;
  if (mode === 'remote') {
    await connectRemoteServer();
    return;
  }
  try {
    isRunning.value = await SysApi.isServerRunning();
    if (isRunning.value) startPolling();
  } catch (error: any) {
    addLog(`[ERROR] ${error?.message || error}`);
  }
};

const startLocalServer = async () => {
  try {
    addLog(t('app.start_server'));
    await SysApi.startServer({ host: serverHost.value, port: serverPort.value });
    isRunning.value = true;
    setTimeout(startPolling, 500);
  } catch (err: any) { addLog(t('app.start_server_fail', { error: err.message || err })); }
};
const stopLocalServer = async () => {
  try {
    addLog(t('app.stop_server'));
    await SysApi.stopServer();
    isRunning.value = false;
    stopPolling();
    addLog(t('app.stop_server_success'));
  } catch (err: any) {
    addLog(t('app.stop_server_fail', { error: err.message || err }));
  }
};

// === Lifecycle ===
onMounted(async () => {
  addLog('[INFO] ' + t('role.dashboard_title') + ' loaded');
  if (isTauri()) {
    unlistenLog = await listen<string>('server_log', (e) => addLog(e.payload));
    unlistenClients = await listen<string>('clients_updated', () => { fetchClients(); fetchUsers(); });
  }
  
  if (props.remoteUrl) {
    remoteServerUrl.value = normalizeServerUrl(props.remoteUrl);
  }
  if (isRemoteMode.value) {
    await connectRemoteServer();
  } else {
    try {
      const running = await SysApi.isServerRunning();
      if (running) {
        isRunning.value = true;
        addLog(t('app.server_running'));
        setTimeout(startPolling, 500);
      }
    } catch (err: any) {
      console.warn("Failed to check server status", err);
    }
  }
});
onUnmounted(() => { unlistenLog?.(); unlistenClients?.(); stopPolling(); });

const openNewClient = async () => {
  try {
    addLog(t('app.start_client'));
    await SysApi.openNewClient();
  } catch (err: any) {
    addLog(t('app.start_client_fail', { error: err.message || err }));
  }
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




</style>
