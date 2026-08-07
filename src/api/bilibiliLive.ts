import { safeInvoke } from './index';

export interface BiliLiveSession {
  sessdata: string;
  bili_jct: string;
  buvid3?: string;
}

export interface LiveRoomInfo {
  room_id: number;
  uid: number;
  title: string;
  area_id: number;
  area_name: string;
  parent_area_id: number;
  parent_area_name: string;
  live_status: number;
  online: number;
  cover: string;
  announcement: string;
}

export interface LiveArea {
  id: number;
  name: string;
  parent_id: number;
  parent_name: string;
  pic: string;
}

export interface StreamEndpoint {
  protocol: string;
  address: string;
  stream_key: string;
  provider: string;
}

export interface LiveStartResult {
  live: boolean;
  requires_face_auth: boolean;
  face_auth_url?: string | null;
  message: string;
  endpoints: StreamEndpoint[];
}

export interface ContributionRankItem {
  uid: number;
  name: string;
  face: string;
  rank: number;
  score: number;
}

export const BilibiliLiveApi = {
  getRoomInfo: (session: BiliLiveSession, roomId: number) =>
    safeInvoke<LiveRoomInfo>('bili_live_get_room_info', { session, roomId }),
  getOwnRoom: (session: BiliLiveSession) =>
    safeInvoke<LiveRoomInfo>('bili_live_get_own_room', { session }),
  getAreas: () => safeInvoke<LiveArea[]>('bili_live_get_areas'),
  updateTitle: (session: BiliLiveSession, roomId: number, title: string) =>
    safeInvoke<void>('bili_live_update_title', { session, roomId, title }),
  updateArea: (session: BiliLiveSession, roomId: number, areaId: number) =>
    safeInvoke<void>('bili_live_update_area', { session, roomId, areaId }),
  updateAnnouncement: (session: BiliLiveSession, roomId: number, uid: number, announcement: string) =>
    safeInvoke<void>('bili_live_update_announcement', { session, roomId, uid, announcement }),
  start: (session: BiliLiveSession, roomId: number, areaId: number) =>
    safeInvoke<LiveStartResult>('bili_live_start', { session, roomId, areaId }),
  stop: (session: BiliLiveSession, roomId: number) =>
    safeInvoke<void>('bili_live_stop', { session, roomId }),
  sendDanmaku: (session: BiliLiveSession, roomId: number, message: string) =>
    safeInvoke<void>('bili_live_send_danmaku', { session, roomId, message }),
  getContributionRank: (session: BiliLiveSession, roomId: number) =>
    safeInvoke<ContributionRankItem[]>('bili_live_get_contribution_rank', { session, roomId }),
};
