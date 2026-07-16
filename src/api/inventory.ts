import { request } from './request';

export const InventoryApi = {
  getUserInventoryItem: (params: { userId: string; inventoryId: string }) =>
    request(`/user/${params.userId}/inventory/${params.inventoryId}`, { method: 'GET' }),

  getInventoryItem: (params: { inventoryId: string }) =>
    request(`/inventory/${params.inventoryId}`, { method: 'GET' }),

  getInventoryItems: (params: {
    n?: number;
    offset?: number;
    order?: string;
    types?: string;
    flags?: string;
    notFlags?: string;
    archived?: boolean;
  } = {}) =>
    request('/inventory', { method: 'GET', params }),

  consumeInventoryBundle: (params: { inventoryId: string }) =>
    request(`/inventory/${params.inventoryId}/consume`, { method: 'PUT', params }),

  getInventoryTemplate: (params: { inventoryTemplateId: string }) =>
    request(`/inventory/template/${params.inventoryTemplateId}`, { method: 'GET' }),

  redeemReward: (params: { code: string }) =>
    request('/reward/redeem', { method: 'POST', params }),

  getGlobalInventory: () =>
    request('/inventory/global', { method: 'GET' }),

  getEquipSlot: (params: { equipSlot: 'drone' | 'warp' | 'portal' | 'loadingscreen'; holderId: string }) =>
    request('/inventory', { method: 'GET', params }),

  equipItem: (params: { equipSlot: 'drone' | 'warp' | 'portal' | 'loadingscreen'; inventoryId: string }) =>
    request(`/inventory/${params.inventoryId}/equip`, { method: 'PUT', params }),

  archiveItem: (params: { inventoryId: string }) =>
    request(`/inventory/${params.inventoryId}`, { method: 'PUT', params: { isArchived: true } }),

  unarchiveItem: (params: { inventoryId: string }) =>
    request(`/inventory/${params.inventoryId}`, { method: 'PUT', params: { isArchived: false } }),

  unArchiveItem: (params: { inventoryId: string }) =>
    request(`/inventory/${params.inventoryId}`, { method: 'PUT', params: { isArchived: false } }),
};
