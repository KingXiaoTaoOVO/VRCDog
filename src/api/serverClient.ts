const SERVER_BASE = '/';

function getServerUrl(path: string): string {
  return `${SERVER_BASE}${path.replace(/^\/+/, '')}`;
}

let adminToken: string | null = null;

export function setAdminToken(token: string | null) {
  adminToken = token;
}

export function getAdminToken(): string | null {
  return adminToken;
}

async function serverFetch<T = any>(
  path: string,
  options: {
    method?: string;
    headers?: Record<string, string>;
    body?: any;
    query?: Record<string, any>;
  } = {}
): Promise<T> {
  const url = new URL(getServerUrl(path), window.location.origin);
  if (options.query) {
    for (const [key, value] of Object.entries(options.query)) {
      if (value !== undefined && value !== null) {
        url.searchParams.append(key, String(value));
      }
    }
  }

  const headers: Record<string, string> = {
    ...options.headers,
  };

  if (adminToken) {
    headers['x-vrcdog-admin-token'] = adminToken;
  }

  const fetchOptions: RequestInit = {
    method: options.method || 'GET',
    headers,
  };

  if (options.body && options.method !== 'GET') {
    if (typeof options.body === 'object' && !(options.body instanceof FormData)) {
      fetchOptions.body = JSON.stringify(options.body);
      headers['Content-Type'] = 'application/json';
    } else {
      fetchOptions.body = options.body as any;
    }
  }

  const response = await fetch(url.toString(), fetchOptions);

  if (!response.ok) {
    const text = await response.text().catch(() => '');
    let message = `HTTP ${response.status}`;
    try {
      const parsed = JSON.parse(text);
      message = parsed.message || parsed.error?.message || message;
    } catch {
      if (text) message = text;
    }
    throw new Error(message);
  }

  const contentType = response.headers.get('content-type') || '';
  if (contentType.includes('application/json')) {
    return await response.json();
  }

  const text = await response.text();
  try {
    return JSON.parse(text);
  } catch {
    return text as unknown as T;
  }
}

export const ServerApi = {
  adminAuth: (password: string) =>
    serverFetch<{ success: boolean; token?: string; message?: string }>('/api/admin/auth', {
      method: 'POST',
      body: { password },
    }),

  getClients: () => serverFetch<{ clients: any[] }>('/api/admin/clients'),
  getUsers: () => serverFetch<{ users: any[]; bans: any[]; frozen: any[] }>('/api/admin/users'),
  kickUser: (userId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/kick', {
      method: 'POST',
      body: { user_id: userId },
    }),
  banUser: (userId: string, reason?: string, durationHours?: number) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/ban', {
      method: 'POST',
      body: { user_id: userId, reason: reason || '', duration_hours: durationHours },
    }),
  unbanUser: (userId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/unban', {
      method: 'POST',
      body: { user_id: userId },
    }),
  freezeUser: (userId: string, reason?: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/freeze', {
      method: 'POST',
      body: { user_id: userId, reason: reason || '' },
    }),
  unfreezeUser: (userId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/unfreeze', {
      method: 'POST',
      body: { user_id: userId },
    }),
  removeUser: (userId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/remove', {
      method: 'POST',
      body: { user_id: userId },
    }),

  getRoles: () => serverFetch<{ roles: any[] }>('/api/admin/roles'),
  saveRole: (role: any) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/roles', {
      method: 'POST',
      body: role,
    }),
  deleteRole: (roleId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/roles/delete', {
      method: 'POST',
      body: { role_id: roleId },
    }),
  setDefaultRole: (roleId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/roles/set_default', {
      method: 'POST',
      body: { role_id: roleId },
    }),
  setUserRole: (userId: string, roleId: string | null) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/users/set_role', {
      method: 'POST',
      body: { user_id: userId, role_id: roleId },
    }),

  getSurveySettings: () => serverFetch('/api/admin/survey-settings'),
  saveSurveySettings: (settings: any) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/survey-settings', {
      method: 'POST',
      body: settings,
    }),
  getSurveys: () => serverFetch<{ surveys: any[] }>('/api/admin/surveys'),
  saveSurvey: (survey: any) =>
    serverFetch<{ success: boolean; message?: string; survey_id?: string; revision?: number }>('/api/admin/surveys', {
      method: 'POST',
      body: survey,
    }),
  publishSurvey: (surveyId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/surveys/publish', {
      method: 'POST',
      body: { survey_id: surveyId },
    }),
  resendSurvey: (surveyId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/surveys/resend', {
      method: 'POST',
      body: { survey_id: surveyId },
    }),
  deleteSurvey: (surveyId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/surveys/delete', {
      method: 'POST',
      body: { survey_id: surveyId },
    }),
  getSurveySubmissions: () => serverFetch<{ submissions: any[] }>('/api/admin/survey-submissions'),
  getSurveyClicks: (surveyId?: string) =>
    serverFetch<{ clicks: any[] }>('/api/admin/survey-clicks', {
      query: surveyId ? { survey_id: surveyId } : undefined,
    }),
  deleteSubmission: (submissionId: string) =>
    serverFetch<{ success: boolean; message?: string }>('/api/admin/survey-submissions/delete', {
      method: 'POST',
      body: { submission_id: submissionId },
    }),

  proxyVrchatApi: async (path: string, method = 'GET', headers: Record<string, string> = {}, body?: any): Promise<any> => {
    const url = new URL(getServerUrl('/api/vrchat-proxy'), window.location.origin);
    url.searchParams.set('path', path);

    const fetchOptions: RequestInit = {
      method,
      headers: {
        ...headers,
      } as Record<string, string>,
      credentials: 'include',
    };

    if (body && method !== 'GET') {
      fetchOptions.body = typeof body === 'string' ? body : JSON.stringify(body);
      const fetchHeaders = fetchOptions.headers as Record<string, string>;
      if (!fetchHeaders['Content-Type']) {
        fetchHeaders['Content-Type'] = 'application/json';
      }
    }

    const response = await fetch(url.toString(), fetchOptions);

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      let message = `HTTP ${response.status}`;
      try {
        const parsed = JSON.parse(text);
        message = parsed.message || parsed.error?.message || message;
      } catch {
        if (text) message = text;
      }
      const err: any = new Error(message);
      err.status = response.status;
      throw err;
    }

    const contentType = response.headers.get('content-type') || '';
    if (contentType.includes('application/json')) {
      return await response.json();
    }

    const text = await response.text();
    try {
      return JSON.parse(text);
    } catch {
      return text;
    }
  },
};

export const DbApi = {
  getAllSettings: async () => {
    try {
      const raw = localStorage.getItem('vrcdog_web_settings');
      if (raw) {
        return JSON.parse(raw);
      }
    } catch {
      // ignore
    }
    return {};
  },
  getSetting: async ({ key }: { key: string }) => {
    const raw = localStorage.getItem(`vrcdog_setting_${key}`);
    if (raw !== null) return raw;
    const allRaw = localStorage.getItem('vrcdog_web_settings');
    if (allRaw) {
      try {
        const all = JSON.parse(allRaw);
        return all[key] || null;
      } catch {
        // ignore
      }
    }
    return null;
  },
  saveSetting: async ({ key, value }: { key: string; value: string }) => {
    const allRaw = localStorage.getItem('vrcdog_web_settings');
    const all = allRaw ? JSON.parse(allRaw) : {};
    all[key] = value;
    localStorage.setItem('vrcdog_web_settings', JSON.stringify(all));
  },
  getAuth: async () => {
    return localStorage.getItem('vrcdog_auth');
  },
  saveAuth: async ({ cookie }: { cookie: string }) => {
    localStorage.setItem('vrcdog_auth', cookie);
  },
  clearAuth: async () => {
    localStorage.removeItem('vrcdog_auth');
  },
  getStatusPresets: async () => {
    try {
      return JSON.parse(localStorage.getItem('vrcdog_status_presets') || '[]');
    } catch {
      return [];
    }
  },
};
