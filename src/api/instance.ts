import { request } from './request';

export const InstanceApi = {
  getInstance: (params: { worldId: string; instanceId: string }) =>
    request(`/instances/${params.worldId}:${params.instanceId}`, { method: 'GET' }),

  createInstance: (params: Record<string, unknown>) =>
    request('/instances', { method: 'POST', params }),

  getInstanceShortName: (instance: { worldId: string; instanceId: string; shortName?: string }) =>
    request(`/instances/${instance.worldId}:${instance.instanceId}/shortName`, {
      method: 'GET',
      params: instance.shortName ? { shortName: instance.shortName } : undefined,
    }),

  getInstanceFromShortName: (params: { shortName: string }) =>
    request(`/instances/s/${params.shortName}`, { method: 'GET' }),

  selfInvite: (instance: { worldId: string; instanceId: string; shortName?: string }) =>
    request(`/invite/myself/to/${instance.worldId}:${instance.instanceId}`, {
      method: 'POST',
      params: instance.shortName ? { shortName: instance.shortName } : undefined,
    }),
};
