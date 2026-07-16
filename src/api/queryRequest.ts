import { AvatarApi } from './avatar';
import { FavoriteApi } from './favorite';
import { FileApi } from './file';
import { GroupApi } from './group';
import { InventoryApi } from './inventory';
import { MiscApi } from './misc';
import { UserApi } from './user';
import { WorldApi } from './world';

const registry: Record<string, (params: any) => Promise<any>> = {
  user: (params) => UserApi.getUser(params),
  'user.dialog': (params) => UserApi.getUser(params),
  'user.force': (params) => UserApi.getUser(params),
  avatar: (params) => AvatarApi.getAvatar(params),
  'avatar.dialog': (params) => AvatarApi.getAvatar(params),
  world: (params) => WorldApi.getWorld(params),
  'world.dialog': (params) => WorldApi.getWorld(params),
  'world.location': (params) => WorldApi.getWorld(params),
  'world.force': (params) => WorldApi.getWorld(params),
  worldsByUser: (params) => WorldApi.getWorlds(params, params?.option),
  group: (params) => GroupApi.getGroup(params),
  'group.dialog': (params) => GroupApi.getGroup(params),
  'group.force': (params) => GroupApi.getGroup(params),
  groupMember: (params) => GroupApi.getGroupMember(params),
  groupMembers: (params) => GroupApi.getGroupMembers(params),
  groupGallery: (params) => GroupApi.getGroupGallery(params),
  groupCalendar: (params) => GroupApi.getGroupCalendar(params),
  groupCalendarEvent: (params) => GroupApi.getGroupCalendarEvent(params),
  avatarGallery: (params) => AvatarApi.getAvatarGallery(params.avatarId),
  favoriteLimits: () => FavoriteApi.getFavoriteLimits(),
  userInventoryItem: (params) => InventoryApi.getUserInventoryItem(params),
  fileAnalysis: (params) => MiscApi.getFileAnalysis(params),
  worldPersistData: (params) => MiscApi.hasWorldPersistData(params),
  mutualCounts: (params) => UserApi.getMutualCounts(params),
  visits: () => MiscApi.getVisits(),
  file: (params) => FileApi.getFile(params),
  avatarStyles: () => AvatarApi.getAvailableAvatarStyles(),
  representedGroup: (params) => GroupApi.getRepresentedGroup(params),
  vrchatCredits: (params) => MiscApi.getVRChatCredits(params),
};

export const QueryRequestApi = {
  async fetch<T = any>(resource: string, params: Record<string, unknown> = {}): Promise<T & { cache: boolean }> {
    const queryFn = registry[resource];
    if (!queryFn) {
      throw new Error(`Unknown query resource: ${resource}`);
    }
    const data = await queryFn(params);
    return {
      ...(data as T),
      cache: false,
    };
  },
};
