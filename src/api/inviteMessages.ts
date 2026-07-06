import { request } from './request';

export type InviteMessageType = 'message' | 'response' | 'request' | 'requestResponse' | string;

export const InviteMessagesApi = {
  getInviteMessages: (params: { userId: string; messageType: InviteMessageType }) =>
    request(`/message/${params.userId}/${params.messageType}`, { method: 'GET' }),

  editInviteMessage: (params: {
    userId: string;
    messageType: InviteMessageType;
    slot: number | string;
    message: string;
  }) =>
    request(`/message/${params.userId}/${params.messageType}/${params.slot}`, {
      method: 'PUT',
      params: { message: params.message },
    }),
};
