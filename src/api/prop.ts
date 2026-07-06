import { request } from './request';

export const PropApi = {
  getProp: (params: { propId: string }) =>
    request(`/props/${params.propId}`, { method: 'GET' }),
};
