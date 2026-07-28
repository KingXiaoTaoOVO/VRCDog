/**
 * midishow CLI 包装器
 * 通过命令行调用，供 Python GUI 使用
 * 用法：
 *   node cli.js search <关键词>
 *   node cli.js download <ID>
 *   node cli.js info <ID>
 */

import { MidiShowDownloader } from './midishow-downloader';
import * as fs from 'node:fs';
import * as path from 'node:path';

const downloader = new MidiShowDownloader();
const BASE_URL = 'https://www.midishow.com';

async function main() {
    const command = process.argv[2];
    const arg = process.argv[3];
    const arg2 = process.argv[4]; // 登录用: password

    if (!command || !arg) {
        console.error('用法:');
        console.error('  node cli.js search <关键词>');
        console.error('  node cli.js download <ID>');
        console.error('  node cli.js info <ID>');
        console.error('  node cli.js login <用户名/邮箱> <密码>');
        process.exit(1);
    }

    switch (command) {
        case 'search':
            await doSearch(arg);
            break;
        case 'download':
            await doDownload(parseInt(arg));
            break;
        case 'info':
            await doInfo(parseInt(arg));
            break;
        case 'login':
            await doLogin(arg, arg2 || '');
            break;
        default:
            console.error(`未知命令: ${command}`);
            process.exit(1);
    }
}

/**
 * 搜索 MIDI 文件
 */
async function doSearch(keyword: string) {
    try {
        // 先获取首页 cookie
        const axios = (await import('axios')).default;
        await axios.get(BASE_URL, {
            headers: {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'
            }
        });

        // 搜索
        const { wrapper } = await import('axios-cookiejar-support');
        const { CookieJar } = await import('tough-cookie');
        const cookieJar = new CookieJar();
        const client = wrapper(axios.create({
            jar: cookieJar,
            withCredentials: true,
            headers: {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
                'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
                'Accept-Language': 'zh-CN,zh;q=0.9,en;q=0.8',
            }
        }));

        const resp = await client.get(`${BASE_URL}/search/result`, {
            params: { q: keyword, page: 1, 'per-page': 30 },
            timeout: 15000,
        });

        const html = resp.data;
        const { JSDOM } = await import('jsdom');
        const dom = new JSDOM(html);
        const doc = dom.window.document;

        const results: any[] = [];

        // 查找所有 midi 卡片
        const cards = doc.querySelectorAll('[data-key], .midi-item, .col-midi, div[class*="midi"]');
        const seen = new Set<number>();

        // 方法1: 从 data-key 属性提取
        cards.forEach((card: Element) => {
            const dataKey = card.getAttribute('data-key');
            if (dataKey) {
                const id = parseInt(dataKey);
                if (!isNaN(id) && !seen.has(id)) {
                    seen.add(id);
                    const titleEl = card.querySelector('a[href*="/en/midi/"], a[href*="/midi/"]');
                    const title = titleEl ? titleEl.textContent?.trim() || `MIDI #${id}` : `MIDI #${id}`;
                    const artistEl = card.querySelector('.artist, .author, [class*="artist"]');
                    const artist = artistEl ? artistEl.textContent?.trim() || '' : '';
                    results.push({ id, title, artist, page_url: `${BASE_URL}/en/midi/${id}.html` });
                }
            }
        });

        // 方法2: 从 HTML 中提取所有 midi ID
        if (results.length === 0) {
            const idPattern = /\/en\/midi\/(\d+)\.html/g;
            let match;
            while ((match = idPattern.exec(html)) !== null) {
                const id = parseInt(match[1]);
                if (!seen.has(id)) {
                    seen.add(id);
                    results.push({
                        id,
                        title: `MIDI #${id}`,
                        artist: '',
                        page_url: `${BASE_URL}/en/midi/${id}.html`,
                    });
                }
            }
        }

        console.log(JSON.stringify(results));
    } catch (err: any) {
        console.error(JSON.stringify({ error: err.message || String(err) }));
        process.exit(1);
    }
}

/**
 * 下载 MIDI 文件
 */
async function doDownload(id: number) {
    try {
        const midiData = await downloader.getMidiFile(id);
        // 输出 base64 编码
        const base64 = Buffer.from(midiData, 'binary').toString('base64');
        console.log(JSON.stringify({ id, data: base64 }));
    } catch (err: any) {
        console.error(JSON.stringify({ error: err.message || String(err) }));
        process.exit(1);
    }
}

/**
 * 获取 MIDI 信息
 */
async function doInfo(id: number) {
    try {
        const axios = (await import('axios')).default;
        const { wrapper } = await import('axios-cookiejar-support');
        const { CookieJar } = await import('tough-cookie');
        const cookieJar = new CookieJar();
        const client = wrapper(axios.create({
            jar: cookieJar,
            withCredentials: true,
            headers: {
                'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
                'Accept-Language': 'zh-CN,zh;q=0.9',
            }
        }));

        const resp = await client.get(`${BASE_URL}/en/midi/${id}.html`, { timeout: 15000 });
        const html = resp.data;
        const { JSDOM } = await import('jsdom');
        const dom = new JSDOM(html);
        const doc = dom.window.document;

        const titleEl = doc.querySelector('h1') || doc.querySelector('title');
        let title = titleEl ? titleEl.textContent?.trim() || '' : `MIDI #${id}`;
        title = title.replace(/\s*-\s*MidiShow.*$/, '').trim();

        const artistEl = doc.querySelector('.artist, .author, [class*="artist"], [class*="author"]');
        const artist = artistEl ? artistEl.textContent?.trim().replace(/^Artist[:：\s]*/i, '') : '';

        const uploaderEl = doc.querySelector('.uploader, [class*="uploader"]');
        const uploader = uploaderEl ? uploaderEl.textContent?.trim().replace(/^Uploader[:：\s]*/i, '') : '';

        console.log(JSON.stringify({
            id,
            title,
            artist,
            uploader,
            page_url: `${BASE_URL}/en/midi/${id}.html`,
        }));
    } catch (err: any) {
        console.error(JSON.stringify({ error: err.message || String(err) }));
        process.exit(1);
    }
}

/**
 * 登录 midishow
 */
async function doLogin(identity: string, password: string) {
    try {
        await downloader.login(identity, password);
        console.log(JSON.stringify({ success: true, message: '登录成功' }));
    } catch (err: any) {
        console.error(JSON.stringify({ error: err.message || String(err) }));
        process.exit(1);
    }
}

main().catch(err => {
    console.error(JSON.stringify({ error: err.message || String(err) }));
    process.exit(1);
});