import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

let prompts = [];
let selectedIds = new Set();
let ctxPromptId = null;

const tagsEl = document.getElementById('tags');
const ctxMenu = document.getElementById('ctx-menu');
const toastEl = document.getElementById('toast');
let toastTimer;

// ── Load & Render ──

async function loadPrompts() {
  try {
    prompts = await invoke('get_prompts_sorted');
    render();
  } catch (e) {
    console.error('[Q-Prompt] Failed to load prompts:', e);
  }
}

function render() {
  tagsEl.innerHTML = prompts.map((p, i) => {
    const num = i + 1;
    const name = p.name.length > 4 ? p.name.slice(0, 4) + '…' : p.name;
    const sel = selectedIds.has(p.id) ? ' selected' : '';
    const dot = selectedIds.has(p.id) ? '<span class="dot">●</span>' : '';
    return `<span class="prompt-tag${sel}" data-id="${p.id}" data-idx="${i}" title="${p.name}">${num}${name}</span>${dot}`;
  }).join('');
}

// ── Click: insert ──

tagsEl.addEventListener('click', async (e) => {
  const tag = e.target.closest('.prompt-tag');
  if (!tag) return;
  const id = tag.dataset.id;

  if (e.shiftKey) {
    e.preventDefault();
    if (selectedIds.has(id)) selectedIds.delete(id);
    else selectedIds.add(id);
    render();
    return;
  }

  if (selectedIds.size > 0 && selectedIds.has(id)) {
    await insertMulti(Array.from(selectedIds));
    selectedIds.clear();
    render();
    return;
  }

  selectedIds.clear();
  const prompt = prompts.find(p => p.id === id);
  if (prompt) await insertPrompt(prompt);
  render();
});

// ── Right-click: context menu ──

tagsEl.addEventListener('contextmenu', (e) => {
  const tag = e.target.closest('.prompt-tag');
  if (!tag) return;
  e.preventDefault();
  ctxPromptId = tag.dataset.id;
  ctxMenu.innerHTML = `
    <div class="ctx-item" data-action="edit">编辑</div>
    <div class="ctx-item" data-action="delete">删除</div>
  `;
  ctxMenu.style.display = 'block';
  ctxMenu.style.left = e.clientX + 'px';
  ctxMenu.style.top = e.clientY + 'px';
});

ctxMenu.addEventListener('click', async (e) => {
  const item = e.target.closest('.ctx-item');
  if (!item) return;
  ctxMenu.style.display = 'none';
  const action = item.dataset.action;
  if (action === 'edit' && ctxPromptId) {
    openManager();
  } else if (action === 'delete' && ctxPromptId) {
    try {
      await invoke('delete_prompt', { id: ctxPromptId });
      await loadPrompts();
    } catch (err) {
      console.error('[Q-Prompt] Delete failed:', err);
    }
  }
  ctxPromptId = null;
});

document.addEventListener('click', () => { ctxMenu.style.display = 'none'; });

// ── Menu button ──

document.getElementById('menu-btn').addEventListener('click', () => openManager());

// ── Insert logic ──

async function insertPrompt(prompt) {
  let text = prompt.content;

  // {{selection}}
  if (text.includes('{{selection}}')) {
    try {
      const sel = await invoke('capture_selection');
      if (sel && sel.length > 0) {
        text = text.replace(/\{\{selection\}\}/g, sel);
      } else {
        const val = prompt('请输入选中文本（用于 {{selection}}）：', '');
        if (val === null) return;
        text = text.replace(/\{\{selection\}\}/g, val);
      }
    } catch (e) {
      console.error('[Q-Prompt] capture_selection failed:', e);
      const val = prompt('请输入选中文本（用于 {{selection}}）：', '');
      if (val === null) return;
      text = text.replace(/\{\{selection\}\}/g, val);
    }
  }

  // {{variable}}
  const vars = text.match(/\{\{(.+?)\}\}/g);
  if (vars) {
    for (const v of vars) {
      const name = v.slice(2, -2);
      const val = prompt(`请填写: ${name}`, '');
      if (val === null) return;
      text = text.replace(v, val);
    }
  }

  // Atomic insert (clipboard + Ctrl+V, bar hides/shows automatically)
  try {
    await invoke('insert_text', { text });
    await invoke('record_usage', { id: prompt.id });
    await loadPrompts();
  } catch (e) {
    console.error('[Q-Prompt] insert_text failed:', e);
    showToast('插入失败: ' + e);
  }
}

async function insertMulti(ids) {
  const texts = [];
  for (const id of ids) {
    const p = prompts.find(p => p.id === id);
    if (p) texts.push(p.content);
  }
  const combined = texts.join('\n---\n');
  try {
    await invoke('insert_text', { text: combined });
    for (const id of ids) {
      await invoke('record_usage', { id });
    }
    await loadPrompts();
  } catch (e) {
    console.error('[Q-Prompt] insertMulti failed:', e);
    showToast('插入失败: ' + e);
  }
}

// ── Manager ──

async function openManager() {
  await invoke('open_manager_window');
}

// ── Shortcut events from Rust ──

listen('shortcut-insert', async (event) => {
  console.log('[Q-Prompt] shortcut-insert event:', event.payload);
  const idx = event.payload;
  if (idx >= 0 && idx < prompts.length) {
    selectedIds.clear();
    await insertPrompt(prompts[idx]);
    render();
  }
});

listen('shortcut-quick-save', async () => {
  console.log('[Q-Prompt] shortcut-quick-save event');
  try {
    const sel = await invoke('capture_selection');
    if (!sel || sel.length === 0) {
      showToast('未选中文本');
      return;
    }
    const name = prompt('提示词名称（2-4字）：', sel.slice(0, 4));
    if (!name) return;
    const content = prompt('提示词内容（可编辑）：', sel);
    if (!content) return;
    await invoke('quick_save', { name, content });
    await loadPrompts();
    showToast('已保存: ' + name);
  } catch (e) {
    console.error('[Q-Prompt] quick_save failed:', e);
  }
});

// ── Toast ──

function showToast(msg) {
  toastEl.textContent = msg;
  toastEl.style.display = 'block';
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { toastEl.style.display = 'none'; }, 3000);
}

// ── Init ──
await loadPrompts();
const win = getCurrentWindow();
await win.show();
console.log('[Q-Prompt] Quick bar ready, prompts:', prompts.length);
