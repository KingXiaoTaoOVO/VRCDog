 
export interface VrcUser {
  id: string;
  displayName: string;
  status?: string;
  statusDescription?: string;
  tags?: string[];
  last_login?: string;
  bio?: string;
  bioLinks?: string[];
  currentAvatarImageUrl?: string;
  currentAvatarThumbnailImageUrl?: string;
  location?: string;
  isFriend?: boolean;
  [key: string]: any;
}

export interface VrcWorld {
  id: string;
  name: string;
  authorName: string;
  description: string;
  capacity?: number;
  favorites?: number;
  visits?: number;
  tags?: string[];
  imageUrl?: string;
  thumbnailImageUrl?: string;
  [key: string]: any;
}

export interface VrcAvatar {
  id: string;
  name: string;
  description: string;
  authorId: string;
  authorName: string;
  imageUrl: string;
  thumbnailImageUrl: string;
  releaseStatus: string;
  supportedPlatforms: string[];
  [key: string]: any;
}

export interface VrcNotification {
  id: string;
  type: string;
  message: string;
  details: string | Record<string, any>;
  created_at: string;
  [key: string]: any;
}

export interface GalleryImage {
  id?: string;
  path: string;
  created_at: number;
  assetUrl?: string;
  dateStr?: string;
  [key: string]: any;
}

export interface FriendLog {
  id: string;
  user_id: string;
  display_name: string;
  event_type: string;
  detail?: string;
  created_at: string;
  [key: string]: any;
}

export interface VrcGroup {
  id: string;
  name: string;
  shortCode: string;
  description?: string;
  memberCount?: number;
  iconUrl?: string;
  bannerUrl?: string;
  [key: string]: any;
}
