import type { AppLocale } from '../languages';

type Messages = Record<string, unknown>;

const localeOrder = ['cs', 'es', 'fr', 'hu', 'ko', 'pl', 'pt', 'ru', 'th', 'vi', 'zh-TW'] as const;
type NativeLocale = typeof localeOrder[number];

// High-visibility application copy shared by the shell, statistics and settings.
// Keeping it in one matrix makes missing languages obvious during review.
const rows: Record<string, readonly string[]> = {
  'app.subtitle': ['Spravce socialnich kontaktu', 'Gestor social', 'Gestionnaire social', 'Kozossegi kezelo', '소셜 매니저', 'Menedzer spolecznosci', 'Gerenciador social', 'Социальный менеджер', 'ตัวจัดการโซเชียล', 'Trinh quan ly xa hoi', '社交管理工具'],
  'app.logout': ['Odhlasit', 'Cerrar sesion', 'Se deconnecter', 'Kijelentkezes', '로그아웃', 'Wyloguj', 'Sair', 'Выйти', 'ออกจากระบบ', 'Dang xuat', '登出'],

  'sidebar.dashboard': ['Prehled', 'Panel', 'Tableau de bord', 'Attekintes', '대시보드', 'Pulpit', 'Painel', 'Обзор', 'ภาพรวม', 'Tong quan', '總覽大屏'],
  'sidebar.feed': ['Aktivity', 'Actividad', 'Activite', 'Hirfolyam', '피드', 'Aktywnosc', 'Atividade', 'Лента', 'ฟีด', 'Bang tin', '動態'],
  'sidebar.friendlog': ['Historie pratel', 'Registro de amigos', 'Journal des amis', 'Baratnaplo', '친구 기록', 'Dziennik znajomych', 'Registro de amigos', 'Журнал друзей', 'บันทึกเพื่อน', 'Nhat ky ban be', '好友記錄'],
  'sidebar.locations': ['Poloha pratel', 'Ubicacion de amigos', 'Position des amis', 'Baratok helye', '친구 위치', 'Lokalizacja znajomych', 'Localizacao de amigos', 'Местоположение друзей', 'ตำแหน่งเพื่อน', 'Vi tri ban be', '好友位置'],
  'sidebar.charts': ['Statistiky', 'Estadisticas', 'Statistiques', 'Statisztikak', '통계', 'Statystyki', 'Estatisticas', 'Статистика', 'สถิติ', 'Thong ke', '數據統計'],
  'sidebar.playerlist': ['Hraci v mistnosti', 'Jugadores de sala', 'Joueurs de la salle', 'Szoba jatekosai', '방 플레이어', 'Gracze w pokoju', 'Jogadores da sala', 'Игроки комнаты', 'ผู้เล่นในห้อง', 'Nguoi choi trong phong', '房間玩家'],
  'sidebar.gallery': ['Galerie', 'Galeria', 'Galerie', 'Galeria', '갤러리', 'Galeria', 'Galeria', 'Галерея', 'แกลเลอรี', 'Thu vien anh', '回憶圖庫'],
  'sidebar.social': ['Komunita', 'Social', 'Social', 'Kozosseg', '소셜', 'Spolecznosc', 'Social', 'Сообщество', 'โซเชียล', 'Xa hoi', '社交'],
  'sidebar.search': ['Hledat', 'Buscar', 'Rechercher', 'Kereses', '검색', 'Szukaj', 'Pesquisar', 'Поиск', 'ค้นหา', 'Tim kiem', '全局搜尋'],
  'sidebar.notifications': ['Oznameni', 'Notificaciones', 'Notifications', 'Ertesitesek', '알림', 'Powiadomienia', 'Notificacoes', 'Уведомления', 'การแจ้งเตือน', 'Thong bao', '通知中心'],
  'sidebar.groups': ['Skupiny', 'Grupos', 'Groupes', 'Csoportok', '그룹', 'Grupy', 'Grupos', 'Группы', 'กลุ่ม', 'Nhom', '群組管理'],
  'sidebar.avatars': ['Moje avatary', 'Mis avatares', 'Mes avatars', 'Avatarjaim', '내 아바타', 'Moje awatary', 'Meus avatares', 'Мои аватары', 'อวตารของฉัน', 'Avatar cua toi', '我的模型'],
  'sidebar.favorites': ['Oblibene', 'Favoritos', 'Favoris', 'Kedvencek', '즐겨찾기', 'Ulubione', 'Favoritos', 'Избранное', 'รายการโปรด', 'Yeu thich', '收藏夾'],
  'sidebar.heatmap': ['Mapa aktivity', 'Mapa de actividad', 'Carte d’activite', 'Aktivitasi terkep', '활동 히트맵', 'Mapa aktywnosci', 'Mapa de atividade', 'Карта активности', 'แผนที่กิจกรรม', 'Ban do hoat dong', '活躍熱力圖'],
  'sidebar.gamelog': ['Herni zaznam', 'Registro del juego', 'Journal du jeu', 'Jateknaplo', '게임 로그', 'Dziennik gry', 'Registro do jogo', 'Журнал игры', 'บันทึกเกม', 'Nhat ky tro choi', '遊戲記錄'],
  'sidebar.notes': ['Poznamky', 'Notas', 'Notes', 'Jegyzetek', '메모', 'Notatki', 'Notas', 'Заметки', 'บันทึก', 'Ghi chu', '備忘錄'],
  'sidebar.presets': ['Predvolby stavu', 'Estados predefinidos', 'Statuts predefinis', 'Allapotkeszletek', '상태 프리셋', 'Ustawienia statusu', 'Status predefinidos', 'Шаблоны статуса', 'สถานะสำเร็จรูป', 'Mau trang thai', '狀態預設'],
  'sidebar.tools': ['Nastroje', 'Herramientas', 'Outils', 'Eszkozok', '도구', 'Narzedzia', 'Ferramentas', 'Инструменты', 'เครื่องมือ', 'Cong cu', '工具箱'],
  'sidebar.translator': ['Prekladac', 'Traductor', 'Traducteur', 'Fordito', '번역기', 'Tlumacz', 'Tradutor', 'Переводчик', 'นักแปล', 'Trinh dich', '翻譯器'],
  'sidebar.remote': ['Vzdalena pomoc', 'Asistencia remota', 'Assistance distante', 'Tavoli segitseg', '원격 지원', 'Pomoc zdalna', 'Assistencia remota', 'Удаленная помощь', 'ช่วยเหลือระยะไกล', 'Ho tro tu xa', '遠端協助'],
  'sidebar.env': ['Spravce prostredi', 'Gestor de entorno', 'Gestionnaire d’environnement', 'Kornyezetkezelo', '환경 관리자', 'Menedzer srodowiska', 'Gerenciador de ambiente', 'Менеджер окружения', 'ตัวจัดการสภาพแวดล้อม', 'Quan ly moi truong', '環境管理'],
  'sidebar.export': ['Export dat', 'Exportar datos', 'Exporter les donnees', 'Adatok exportalasa', '데이터 내보내기', 'Eksport danych', 'Exportar dados', 'Экспорт данных', 'ส่งออกข้อมูล', 'Xuat du lieu', '資料匯出'],
  'sidebar.settings': ['Nastaveni', 'Ajustes', 'Parametres', 'Beallitasok', '설정', 'Ustawienia', 'Configuracoes', 'Настройки', 'การตั้งค่า', 'Cai dat', '設定'],

  'status.online': ['Online', 'En linea', 'En ligne', 'Online', '온라인', 'Online', 'Online', 'В сети', 'ออนไลน์', 'Truc tuyen', '在線'],
  'status.join_me': ['Pridej se', 'Unete', 'Rejoignez-moi', 'Csatlakozz', '참여 가능', 'Dolacz do mnie', 'Junte-se', 'Присоединяйтесь', 'เข้าร่วมได้', 'Tham gia cung toi', '可加入'],
  'status.ask_me': ['Na dotaz', 'Preguntame', 'Demandez-moi', 'Keress meg', '요청 후 참여', 'Zapytaj mnie', 'Pergunte-me', 'Спросите меня', 'สอบถามก่อน', 'Hoi toi', '需詢問'],
  'status.busy': ['Nerusit', 'Ocupado', 'Occupe', 'Elfoglalt', '바쁨', 'Zajety', 'Ocupado', 'Занят', 'ไม่ว่าง', 'Ban', '忙碌'],
  'status.offline': ['Offline', 'Desconectado', 'Hors ligne', 'Offline', '오프라인', 'Offline', 'Offline', 'Не в сети', 'ออฟไลน์', 'Ngoai tuyen', '離線'],
  'status.pipeline_online': ['Sluzba pripojena', 'Servicio conectado', 'Service connecte', 'Szolgaltatas csatlakoztatva', '서비스 연결됨', 'Usluga polaczona', 'Servico conectado', 'Служба подключена', 'เชื่อมต่อบริการแล้ว', 'Dich vu da ket noi', '服務端已連線'],
  'status.pipeline_offline': ['Sluzba odpojena', 'Servicio desconectado', 'Service deconnecte', 'Szolgaltatas levalasztva', '서비스 연결 끊김', 'Usluga rozlaczona', 'Servico desconectado', 'Служба отключена', 'บริการออฟไลน์', 'Dich vu mat ket noi', '服務端已離線'],
  'status.frames': ['{count} FPS', '{count} FPS', '{count} IPS', '{count} FPS', '{count} FPS', '{count} FPS', '{count} FPS', '{count} FPS', '{count} FPS', '{count} FPS', '{count} FPS'],

  'charts.overview': ['Prehled', 'Resumen', 'Apercu', 'Attekintes', '개요', 'Przeglad', 'Visao geral', 'Обзор', 'ภาพรวม', 'Tong quan', '數據概覽'],
  'charts.network': ['Socialni sit', 'Red social', 'Reseau social', 'Kozossegi halo', '소셜 네트워크', 'Siec spoleczna', 'Rede social', 'Социальная сеть', 'เครือข่ายสังคม', 'Mang xa hoi', '社交拓撲網'],
  'charts.top_worlds': ['Oblibene svety', 'Mundos populares', 'Mondes populaires', 'Nepszeru vilagok', '인기 월드', 'Popularne swiaty', 'Mundos populares', 'Популярные миры', 'โลกยอดนิยม', 'The gioi pho bien', '熱門世界'],
  'charts.weekly_trend': ['Tydenni aktivita', 'Actividad semanal', 'Activite de la semaine', 'Heti aktivitas', '주간 활동', 'Aktywnosc tygodniowa', 'Atividade semanal', 'Активность за неделю', 'กิจกรรมรายสัปดาห์', 'Hoat dong trong tuan', '本週活躍趨勢'],
  'charts.recent_events': ['Posledni udalosti', 'Eventos recientes', 'Evenements recents', 'Legutobbi esemenyek', '최근 이벤트', 'Ostatnie zdarzenia', 'Eventos recentes', 'Последние события', 'เหตุการณ์ล่าสุด', 'Su kien gan day', '最近事件'],
  'charts.no_events': ['Zadne zaznamy', 'Sin eventos', 'Aucun evenement', 'Nincs esemeny', '이벤트 기록 없음', 'Brak zdarzen', 'Sem eventos', 'Нет событий', 'ไม่มีเหตุการณ์', 'Khong co su kien', '暫無事件記錄'],
  'charts.events': ['udalosti', 'eventos', 'evenements', 'esemeny', '이벤트', 'zdarzenia', 'eventos', 'события', 'เหตุการณ์', 'su kien', '事件'],
  'charts.total_friends': ['Pratele celkem', 'Total de amigos', 'Total d’amis', 'Osszes barat', '전체 친구', 'Wszyscy znajomi', 'Total de amigos', 'Всего друзей', 'เพื่อนทั้งหมด', 'Tong so ban be', '好友總數'],
  'charts.online': ['Online', 'En linea', 'En ligne', 'Online', '온라인', 'Online', 'Online', 'В сети', 'ออนไลน์', 'Truc tuyen', '在線'],
  'charts.friends': ['pratel', 'amigos', 'amis', 'barat', '친구', 'znajomych', 'amigos', 'друзей', 'เพื่อน', 'ban be', '好友'],
  'charts.friends_in_world': ['pratel zde', 'amigos aqui', 'amis ici', 'barat itt', '명의 친구', 'znajomych tutaj', 'amigos aqui', 'друзей здесь', 'เพื่อนที่นี่', 'ban be tai day', '位好友在內'],
  'charts.top_worlds_title': ['Nejnavstevovanejsi svety', 'Mundos mas visitados', 'Mondes les plus visites', 'Leglatogatottabb vilagok', '친구들이 자주 가는 월드', 'Najczesciej odwiedzane swiaty', 'Mundos mais visitados', 'Самые посещаемые миры', 'โลกที่เพื่อนไปบ่อย', 'The gioi ban be ghe tham nhieu', '好友最常造訪的世界'],
  'charts.unknown_world': ['Neznamy svet', 'Mundo desconocido', 'Monde inconnu', 'Ismeretlen vilag', '알 수 없는 월드', 'Nieznany swiat', 'Mundo desconhecido', 'Неизвестный мир', 'โลกที่ไม่รู้จัก', 'The gioi khong xac dinh', '未知世界'],
  'charts.no_data': ['Zadna data', 'Sin datos', 'Aucune donnee', 'Nincs adat', '데이터 없음', 'Brak danych', 'Sem dados', 'Нет данных', 'ไม่มีข้อมูล', 'Khong co du lieu', '暫無資料'],
  'charts.network_desc': ['Prozkoumejte sve socialni vazby ve VRChat', 'Explora tus conexiones sociales de VRChat', 'Explorez vos liens sociaux VRChat', 'Fedezd fel VRChat kapcsolataidat', 'VRChat 소셜 연결을 살펴보세요', 'Poznaj swoje relacje w VRChat', 'Explore suas conexoes sociais no VRChat', 'Исследуйте свои связи в VRChat', 'สำรวจเครือข่ายสังคม VRChat', 'Kham pha ket noi xa hoi VRChat', '探索你的 VRChat 社交連結'],
  'charts.generate_topology': ['Vytvorit sit', 'Generar red', 'Generer le reseau', 'Halozat letrehozasa', '네트워크 생성', 'Generuj siec', 'Gerar rede', 'Построить сеть', 'สร้างเครือข่าย', 'Tao mang', '生成拓撲圖'],
  'charts.regenerate': ['Obnovit', 'Regenerar', 'Regenerer', 'Ujra', '다시 생성', 'Generuj ponownie', 'Gerar novamente', 'Построить заново', 'สร้างใหม่', 'Tao lai', '重新生成'],

  'settings.title': ['Nastaveni', 'Ajustes', 'Parametres', 'Beallitasok', '설정', 'Ustawienia', 'Configuracoes', 'Настройки', 'การตั้งค่า', 'Cai dat', '設定'],
  'settings.subtitle': ['Upravte si VrcDog podle sebe', 'Personaliza tu experiencia con VrcDog', 'Personnalisez votre experience VrcDog', 'Szabd szemelyre a VrcDogot', 'VrcDog 환경을 맞춤 설정하세요', 'Dostosuj VrcDog do swoich potrzeb', 'Personalize sua experiencia VrcDog', 'Настройте VrcDog под себя', 'ปรับแต่งประสบการณ์ VrcDog', 'Tuy chinh trai nghiem VrcDog', '自訂你的 VrcDog 使用體驗'],
  'settings.nav_general': ['Obecne', 'General', 'General', 'Altalanos', '일반', 'Ogolne', 'Geral', 'Общие', 'ทั่วไป', 'Chung', '一般'],
  'settings.nav_language': ['Jazyk', 'Idioma', 'Langue', 'Nyelv', '언어', 'Jezyk', 'Idioma', 'Язык', 'ภาษา', 'Ngon ngu', '介面語言'],
  'settings.nav_theme': ['Vzhled', 'Apariencia', 'Apparence', 'Megjelenes', '외관', 'Wyglad', 'Aparencia', 'Оформление', 'รูปลักษณ์', 'Giao dien', '外觀'],
  'settings.nav_notifications': ['Oznameni', 'Notificaciones', 'Notifications', 'Ertesitesek', '알림', 'Powiadomienia', 'Notificacoes', 'Уведомления', 'การแจ้งเตือน', 'Thong bao', '通知'],
  'settings.nav_network': ['Sit a proxy', 'Red y proxy', 'Reseau et proxy', 'Halozat es proxy', '네트워크 및 프록시', 'Siec i proxy', 'Rede e proxy', 'Сеть и прокси', 'เครือข่ายและพร็อกซี', 'Mang va proxy', '網路與代理'],
  'settings.nav_storage': ['Uloziste a cache', 'Almacenamiento y cache', 'Stockage et cache', 'Tarhely es gyorsitotar', '저장소 및 캐시', 'Pamiec i cache', 'Armazenamento e cache', 'Хранилище и кэш', 'พื้นที่จัดเก็บและแคช', 'Luu tru va bo nho dem', '儲存與快取'],
  'settings.nav_integration': ['Integrace', 'Integracion', 'Integration', 'Integracio', '연동', 'Integracja', 'Integracao', 'Интеграция', 'การเชื่อมต่อ', 'Tich hop', '整合'],
  'settings.nav_auto_start': ['Automaticke spousteni', 'Inicio automatico', 'Demarrage automatique', 'Automatikus inditas', '자동 시작', 'Autostart', 'Inicio automatico', 'Автозапуск', 'เริ่มอัตโนมัติ', 'Tu dong khoi dong', '自動啟動程式'],
  'settings.nav_advanced': ['Pokrocile', 'Avanzado', 'Avance', 'Halado', '고급', 'Zaawansowane', 'Avancado', 'Дополнительно', 'ขั้นสูง', 'Nang cao', '進階'],
  'settings.nav_security': ['Zabezpeceni', 'Seguridad', 'Securite', 'Biztonsag', '보안', 'Bezpieczenstwo', 'Seguranca', 'Безопасность', 'ความปลอดภัย', 'Bao mat', '安全性'],
  'settings.nav_vr': ['VR prekryti', 'Superposicion VR', 'Superposition VR', 'VR reteg', 'VR 오버레이', 'Nakladka VR', 'Sobreposicao VR', 'VR-оверлей', 'โอเวอร์เลย์ VR', 'Lop phu VR', 'VR 覆蓋層'],
  'settings.nav_ocr': ['Rozpoznavani OCR', 'Reconocimiento OCR', 'Reconnaissance OCR', 'OCR felismeres', 'OCR 인식', 'Rozpoznawanie OCR', 'Reconhecimento OCR', 'Распознавание OCR', 'การรู้จำ OCR', 'Nhan dang OCR', 'OCR 辨識'],
  'settings.nav_translation': ['Prekladovy modul', 'Motor de traduccion', 'Moteur de traduction', 'Forditomotor', '번역 엔진', 'Silnik tlumaczen', 'Mecanismo de traducao', 'Модуль перевода', 'ระบบแปลภาษา', 'Cong cu dich', '翻譯引擎'],
  'settings.language_intro': ['Vyberte jazyk rozhrani', 'Elige el idioma de la interfaz', 'Choisissez la langue de l’interface', 'Valaszd ki a felulet nyelvet', '인터페이스 언어 선택', 'Wybierz jezyk interfejsu', 'Escolha o idioma da interface', 'Выберите язык интерфейса', 'เลือกภาษาของอินเทอร์เฟซ', 'Chon ngon ngu giao dien', '選擇介面語言'],
  'settings.language_hint': ['Zmena se ihned projevi v celem VrcDog. Volba se ulozi lokalne.', 'El cambio se aplica al instante en todo VrcDog y se guarda localmente.', 'Le changement s’applique immediatement a tout VrcDog et reste enregistre.', 'A valtas azonnal ervenyes az egesz VrcDogban, es helyben mentodik.', '변경 사항이 VrcDog 전체에 즉시 적용되고 로컬에 저장됩니다.', 'Zmiana od razu obejmie caly VrcDog i zostanie zapisana lokalnie.', 'A alteracao e aplicada imediatamente em todo o VrcDog e salva localmente.', 'Изменение сразу применяется ко всему VrcDog и сохраняется локально.', 'การเปลี่ยนภาษาจะมีผลกับ VrcDog ทั้งหมดทันทีและบันทึกไว้ในเครื่อง', 'Thay doi duoc ap dung ngay cho toan bo VrcDog va luu cuc bo.', '切換後會立即套用至整個 VrcDog，並儲存在本機。'],
  'settings.save': ['Ulozit nastaveni', 'Guardar ajustes', 'Enregistrer', 'Beallitasok mentese', '설정 저장', 'Zapisz ustawienia', 'Salvar configuracoes', 'Сохранить настройки', 'บันทึกการตั้งค่า', 'Luu cai dat', '儲存設定'],
  'settings.saved': ['Ulozeno', 'Guardado', 'Enregistre', 'Mentve', '저장됨', 'Zapisano', 'Salvo', 'Сохранено', 'บันทึกแล้ว', 'Da luu', '已儲存']
};

const dayRows: Record<NativeLocale, readonly string[]> = {
  cs: ['Po', 'Ut', 'St', 'Ct', 'Pa', 'So', 'Ne'],
  es: ['Lun', 'Mar', 'Mie', 'Jue', 'Vie', 'Sab', 'Dom'],
  fr: ['Lun', 'Mar', 'Mer', 'Jeu', 'Ven', 'Sam', 'Dim'],
  hu: ['He', 'Ke', 'Sze', 'Csu', 'Pe', 'Szo', 'Va'],
  ko: ['월', '화', '수', '목', '금', '토', '일'],
  pl: ['Pon', 'Wt', 'Sr', 'Czw', 'Pt', 'Sob', 'Nd'],
  pt: ['Seg', 'Ter', 'Qua', 'Qui', 'Sex', 'Sab', 'Dom'],
  ru: ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс'],
  th: ['จ.', 'อ.', 'พ.', 'พฤ.', 'ศ.', 'ส.', 'อา.'],
  vi: ['T2', 'T3', 'T4', 'T5', 'T6', 'T7', 'CN'],
  'zh-TW': ['一', '二', '三', '四', '五', '六', '日']
};

const setPath = (target: Messages, path: string, value: unknown) => {
  const parts = path.split('.');
  let cursor = target;
  parts.forEach((part, index) => {
    if (index === parts.length - 1) {
      cursor[part] = value;
      return;
    }
    const next = cursor[part];
    if (!next || typeof next !== 'object' || Array.isArray(next)) cursor[part] = {};
    cursor = cursor[part] as Messages;
  });
};

export const nativeCoreMessages = Object.fromEntries(localeOrder.map((locale, localeIndex) => {
  const messages: Messages = {};
  Object.entries(rows).forEach(([path, values]) => setPath(messages, path, values[localeIndex]));
  setPath(messages, 'charts.days', [...dayRows[locale]]);
  return [locale, messages];
})) as Partial<Record<AppLocale, Messages>>;

// The legacy locale files contain English placeholders for these locales. Keep the
// high-traffic shell copy in real Unicode text so switching language never exposes
// mojibake while the remaining long-tail pages are migrated.
const qualityPatches: Partial<Record<NativeLocale, Record<string, string>>> = {
  ko: {
    'app.subtitle': '소셜 관리자', 'app.logout': '로그아웃',
    'sidebar.dashboard': '대시보드', 'sidebar.locations': '친구 위치', 'sidebar.charts': '통계', 'sidebar.playerlist': '방 참가자', 'sidebar.gallery': '갤러리', 'sidebar.social': '소셜', 'sidebar.search': '전체 검색', 'sidebar.notifications': '알림 센터', 'sidebar.groups': '그룹 관리', 'sidebar.avatars': '내 아바타', 'sidebar.favorites': '즐겨찾기', 'sidebar.heatmap': '활동 히트맵', 'sidebar.notes': '메모', 'sidebar.presets': '상태 프리셋', 'sidebar.tools': '도구', 'sidebar.translator': '번역기', 'sidebar.remote': '원격 지원', 'sidebar.env': '환경 관리자', 'sidebar.export': '데이터 내보내기', 'sidebar.settings': '설정',
    'charts.overview': '개요', 'charts.network': '소셜 네트워크', 'charts.top_worlds': '인기 월드', 'charts.weekly_trend': '주간 활동 추이', 'charts.recent_events': '최근 이벤트', 'charts.events': '이벤트', 'charts.no_data': '데이터 없음', 'charts.no_events': '이벤트 기록 없음', 'charts.top_worlds_title': '가장 많이 방문한 월드', 'charts.network_desc': 'VRChat 소셜 연결을 탐색하세요', 'charts.generate_topology': '네트워크 생성', 'charts.regenerate': '다시 생성',
    'settings.title': '설정', 'settings.subtitle': 'VrcDog 환경과 환경설정을 맞춤 설정하세요', 'settings.nav_general': '일반', 'settings.nav_language': '언어', 'settings.nav_theme': '테마', 'settings.nav_notifications': '알림', 'settings.nav_network': '네트워크 및 프록시', 'settings.nav_storage': '저장소 및 캐시', 'settings.nav_integration': '통합', 'settings.nav_auto_start': '자동 시작', 'settings.nav_advanced': '고급', 'settings.nav_security': '보안', 'settings.language_intro': '인터페이스 언어 선택', 'settings.language_hint': '변경 사항은 VrcDog 전체에 즉시 적용되며 이 기기에 저장됩니다.', 'settings.save': '설정 저장', 'settings.saved': '저장됨'
  },
  ru: {
    'app.subtitle': 'Социальный менеджер', 'app.logout': 'Выйти',
    'sidebar.dashboard': 'Обзор', 'sidebar.locations': 'Местоположение друзей', 'sidebar.charts': 'Статистика', 'sidebar.playerlist': 'Игроки комнаты', 'sidebar.gallery': 'Галерея', 'sidebar.social': 'Социальное', 'sidebar.search': 'Поиск', 'sidebar.notifications': 'Уведомления', 'sidebar.groups': 'Группы', 'sidebar.avatars': 'Мои аватары', 'sidebar.favorites': 'Избранное', 'sidebar.heatmap': 'Тепловая карта активности', 'sidebar.notes': 'Заметки', 'sidebar.presets': 'Пресеты статуса', 'sidebar.tools': 'Инструменты', 'sidebar.translator': 'Переводчик', 'sidebar.remote': 'Удалённая помощь', 'sidebar.env': 'Менеджер среды', 'sidebar.export': 'Экспорт данных', 'sidebar.settings': 'Настройки',
    'charts.overview': 'Обзор', 'charts.network': 'Социальная сеть', 'charts.top_worlds': 'Популярные миры', 'charts.weekly_trend': 'Недельная активность', 'charts.recent_events': 'Последние события', 'charts.events': 'событий', 'charts.no_data': 'Нет данных', 'charts.no_events': 'Нет записей событий', 'charts.top_worlds_title': 'Самые посещаемые миры', 'charts.network_desc': 'Исследуйте социальные связи VRChat', 'charts.generate_topology': 'Создать сеть', 'charts.regenerate': 'Создать заново',
    'settings.title': 'Настройки', 'settings.subtitle': 'Настройте VrcDog под себя', 'settings.nav_general': 'Общие', 'settings.nav_language': 'Язык', 'settings.nav_theme': 'Оформление', 'settings.nav_notifications': 'Уведомления', 'settings.nav_network': 'Сеть и прокси', 'settings.nav_storage': 'Хранилище и кэш', 'settings.nav_integration': 'Интеграция', 'settings.nav_auto_start': 'Автозапуск', 'settings.nav_advanced': 'Расширенные', 'settings.nav_security': 'Безопасность', 'settings.language_intro': 'Выберите язык интерфейса', 'settings.language_hint': 'Изменение сразу применяется ко всему VrcDog и сохраняется локально.', 'settings.save': 'Сохранить настройки', 'settings.saved': 'Сохранено'
  },
  th: {
    'app.subtitle': 'ตัวจัดการโซเชียล', 'app.logout': 'ออกจากระบบ',
    'sidebar.dashboard': 'แดชบอร์ด', 'sidebar.locations': 'ตำแหน่งเพื่อน', 'sidebar.charts': 'สถิติ', 'sidebar.playerlist': 'ผู้เล่นในห้อง', 'sidebar.gallery': 'แกลเลอรี', 'sidebar.social': 'โซเชียล', 'sidebar.search': 'ค้นหาทั้งหมด', 'sidebar.notifications': 'ศูนย์การแจ้งเตือน', 'sidebar.groups': 'จัดการกลุ่ม', 'sidebar.avatars': 'อวาตาร์ของฉัน', 'sidebar.favorites': 'รายการโปรด', 'sidebar.heatmap': 'แผนที่ความร้อนกิจกรรม', 'sidebar.notes': 'บันทึก', 'sidebar.presets': 'พรีเซ็ตสถานะ', 'sidebar.tools': 'เครื่องมือ', 'sidebar.translator': 'ตัวแปลภาษา', 'sidebar.remote': 'ช่วยเหลือระยะไกล', 'sidebar.env': 'ตัวจัดการสภาพแวดล้อม', 'sidebar.export': 'ส่งออกข้อมูล', 'sidebar.settings': 'การตั้งค่า',
    'charts.overview': 'ภาพรวม', 'charts.network': 'เครือข่ายโซเชียล', 'charts.top_worlds': 'โลกยอดนิยม', 'charts.weekly_trend': 'แนวโน้มกิจกรรมรายสัปดาห์', 'charts.recent_events': 'เหตุการณ์ล่าสุด', 'charts.events': 'เหตุการณ์', 'charts.no_data': 'ไม่มีข้อมูล', 'charts.no_events': 'ไม่มีบันทึกเหตุการณ์', 'charts.top_worlds_title': 'โลกที่เพื่อนเข้าชมมากที่สุด', 'charts.network_desc': 'สำรวจความเชื่อมโยงทางสังคมใน VRChat', 'charts.generate_topology': 'สร้างเครือข่าย', 'charts.regenerate': 'สร้างใหม่',
    'settings.title': 'การตั้งค่า', 'settings.subtitle': 'ปรับแต่งประสบการณ์ VrcDog ของคุณ', 'settings.nav_general': 'ทั่วไป', 'settings.nav_language': 'ภาษา', 'settings.nav_theme': 'ธีม', 'settings.nav_notifications': 'การแจ้งเตือน', 'settings.nav_network': 'เครือข่ายและพร็อกซี', 'settings.nav_storage': 'พื้นที่จัดเก็บและแคช', 'settings.nav_integration': 'การผสานรวม', 'settings.nav_auto_start': 'เริ่มอัตโนมัติ', 'settings.nav_advanced': 'ขั้นสูง', 'settings.nav_security': 'ความปลอดภัย', 'settings.language_intro': 'เลือกภาษาของอินเทอร์เฟซ', 'settings.language_hint': 'การเปลี่ยนแปลงมีผลทันทีทั่วทั้ง VrcDog และบันทึกไว้ในเครื่อง', 'settings.save': 'บันทึกการตั้งค่า', 'settings.saved': 'บันทึกแล้ว'
  },
  'zh-TW': {
    'app.subtitle': '社交管理工具', 'app.logout': '登出',
    'sidebar.dashboard': '總覽', 'sidebar.locations': '好友位置', 'sidebar.charts': '資料統計', 'sidebar.playerlist': '房間玩家', 'sidebar.gallery': '相簿', 'sidebar.social': '社交', 'sidebar.search': '全域搜尋', 'sidebar.notifications': '通知中心', 'sidebar.groups': '群組管理', 'sidebar.avatars': '我的模型', 'sidebar.favorites': '收藏夾', 'sidebar.heatmap': '活動熱力圖', 'sidebar.notes': '筆記', 'sidebar.presets': '狀態預設', 'sidebar.tools': '工具', 'sidebar.translator': '翻譯器', 'sidebar.remote': '遠端協助', 'sidebar.env': '環境管理員', 'sidebar.export': '資料匯出', 'sidebar.settings': '設定',
    'charts.overview': '資料概覽', 'charts.network': '社交拓撲網', 'charts.top_worlds': '熱門世界', 'charts.weekly_trend': '每週活動趨勢', 'charts.recent_events': '最近事件', 'charts.events': '事件', 'charts.no_data': '暫無資料', 'charts.no_events': '沒有事件紀錄', 'charts.top_worlds_title': '好友最常造訪的世界', 'charts.network_desc': '探索你的 VRChat 社交連結', 'charts.generate_topology': '產生拓撲網', 'charts.regenerate': '重新產生',
    'settings.title': '設定', 'settings.subtitle': '自訂你的 VrcDog 體驗與偏好', 'settings.nav_general': '一般', 'settings.nav_language': '語言', 'settings.nav_theme': '外觀', 'settings.nav_notifications': '通知', 'settings.nav_network': '網路與代理伺服器', 'settings.nav_storage': '儲存空間與快取', 'settings.nav_integration': '整合', 'settings.nav_auto_start': '自動啟動', 'settings.nav_advanced': '進階', 'settings.nav_security': '安全性', 'settings.language_intro': '選擇介面語言', 'settings.language_hint': '變更會立即套用至整個 VrcDog，並儲存在本機。', 'settings.save': '儲存設定', 'settings.saved': '已儲存'
  }
};

Object.entries(qualityPatches).forEach(([locale, patch]) => {
  const messages = nativeCoreMessages[locale as NativeLocale];
  if (!messages || !patch) return;
  Object.entries(patch).forEach(([path, value]) => setPath(messages, path, value));
});

setPath(nativeCoreMessages.ko!, 'charts.days', ['월', '화', '수', '목', '금', '토', '일']);
setPath(nativeCoreMessages.ru!, 'charts.days', ['Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб', 'Вс']);
setPath(nativeCoreMessages.th!, 'charts.days', ['จ.', 'อ.', 'พ.', 'พฤ.', 'ศ.', 'ส.', 'อา.']);
setPath(nativeCoreMessages['zh-TW']!, 'charts.days', ['一', '二', '三', '四', '五', '六', '日']);
