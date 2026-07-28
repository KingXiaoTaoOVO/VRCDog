import { request } from './request';
import { resolveCurrentUserId } from './utils';

export type InviteMessageType = 'message' | 'response' | 'request' | 'requestResponse' | string;

export const InviteMessagesApi = {
  refreshInviteMessageTableData: async (messageType: InviteMessageType, userId?: string) => {
    const resolvedUserId = await resolveCurrentUserId(userId);
    return request(`/message/${resolvedUserId}/${messageType}`, { method: 'GET' });
  },

  getInviteMessages: async (params: { userId?: string; messageType: InviteMessageType }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/message/${userId}/${params.messageType}`, { method: 'GET' });
  },

  editInviteMessage: async (params: {
    userId?: string;
    messageType: InviteMessageType;
    slot: number | string;
    message: string;
  }) => {
    const userId = await resolveCurrentUserId(params.userId);
    return request(`/message/${userId}/${params.messageType}/${params.slot}`, {
      method: 'PUT',
      params: { message: params.message },
    });
  },
};
