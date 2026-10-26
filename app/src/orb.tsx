// 数字财务形象 v4：纯眼表情系统（无嘴无腮红）· 眨眼 · 丰富表情 · 抬头低头 · 果冻
import { useEffect, useRef, useState } from 'react';

export type Persona = 'gentle' | 'boss';

// 颜色方案：以极光色相漂移表达（hue-rotate）+ 光晕色
const COLORS: Record<string, { hue: number; glow: string }> = {
  虹彩: { hue: 0, glow: 'rgba(120, 130, 190, 0.25)' },
  星紫: { hue: 40, glow: 'rgba(142, 123, 240, 0.30)' },
  雾蓝: { hue: -30, glow: 'rgba(127, 159, 232, 0.28)' },
  薄荷: { hue: -70, glow: 'rgba(124, 196, 168, 0.26)' },
  暖砂: { hue: 120, glow: 'rgba(208, 176, 136, 0.26)' },
};

const BUBBLES = ['遇到问题？说说看～', '想算税？点我呀', '有票据疑问吗？', '申报提醒看这里'];

type Mood = 'normal' | 'curious' | 'happy' | 'think' | 'wide';

/** 鼠标追踪：横转 + 抬头/低头（rotateX）+ 眼球位移 */
function useLook(ref: React.RefObject<HTMLElement | null>, range = 320) {
  const [look, setLook] = useState({ x: 0, y: 0, near: false, yaw: 0, pitch: 0 });
  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      const el = ref.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const cx = r.left + r.width / 2;
      const cy = r.top + r.height / 2;
      const dx = e.clientX - cx;
      const dy = e.clientY - cy;
      const dist = Math.hypot(dx, dy);
      if (dist < range) {
        const k = Math.min(1, dist / 120);
        setLook({
          x: (dx / (dist || 1)) * 4.2 * k,
          y: (dy / (dist || 1)) * 3.6 * k,
          near: true,
          yaw: Math.max(-7, Math.min(7, dx / 22)),
          pitch: Math.max(-10, Math.min(10, -dy / 16)), // 鼠标在上→抬头(+)
        });
      } else {
        setLook({ x: 0, y: 0, near: false, yaw: 0, pitch: 0 });
      }
    };
    window.addEventListener('mousemove', onMove, { passive: true });
    return () => window.removeEventListener('mousemove', onMove);
  }, [ref, range]);
  return look;
}

/** 眨眼（随机间隔双眨） */
function useBlink() {
  const [blink, setBlink] = useState(0); // 0 开 1 闭
  useEffect(() => {
    let alive = true;
    let t1: ReturnType<typeof setTimeout>;
    const loop = () => {
      t1 = setTimeout(() => {
        if (!alive) return;
        setBlink(1);
        setTimeout(() => alive && setBlink(0), 120);
        setTimeout(() => {
          if (!alive) return;
          setBlink(1);
          setTimeout(() => alive && setBlink(0), 110);
          loop();
        }, 220);
      }, 2600 + Math.random() * 3800);
    };
    loop();
    return () => { alive = false; clearTimeout(t1); };
  }, []);
  return blink;
}

/** 纯眼表情脸：药丸形眼睛（参考图），无嘴无腮红；表情由眼睛形状表达 */
function JellyFace({
  look, blink, mood, size = 88, glint = true,
}: {
  look: { x: number; y: number; yaw: number; pitch: number };
  blink: number;
  mood: Mood;
  size?: number;
  glint?: boolean; // 悬浮球 false；对话窗 3D 形象 true
}) {
  const s = size / 88;
  // 眼睛形态（emoji 式）——药丸圆角矩形为基底
  const eyeStyle: React.CSSProperties = (() => {
    switch (mood) {
      case 'happy':   // ^ ^ 弯月眯眼
        return { width: 16 * s, height: 15 * s, borderRadius: '50%', transform: `translate(${look.x}px, ${look.y}px) scaleY(${0.3})` };
      case 'wide':    // 惊讶：瞪大拉长
        return { width: 15 * s, height: 26 * s, transform: `translate(${look.x}px, ${look.y}px) scale(1.08)` };
      case 'think':   // 思考：眯起向上瞟
        return { width: 13 * s, height: 13 * s, transform: `translate(${look.x * 0.6}px, ${-2.2 * s}px) scaleY(0.7)` };
      case 'curious': // 好奇：略放大
        return { width: 14.5 * s, height: 24 * s, transform: `translate(${look.x}px, ${look.y}px) scale(1.05)` };
      default:
        return { width: 13.5 * s, height: 22 * s, transform: `translate(${look.x}px, ${look.y}px)` };
    }
  })();

  const blinkStyle: React.CSSProperties = blink ? { transform: `${String(eyeStyle.transform ?? '')} scaleY(0.08)` } : {};
  const merged = {
    borderRadius: `${8 * s}px`,
    ...eyeStyle,
    ...blinkStyle,
  };

  return (
    <div
      className="face"
      style={{
        width: size,
        height: size,
        transform: `perspective(240px) rotateX(${look.pitch}deg) rotate(${look.yaw * 0.35}deg) translateY(${look.y * 0.35}px)`,
      }}
    >
      <div className="face-eye" style={{ left: 20 * s, top: 30 * s, ...merged }}>
        {glint && mood !== 'happy' && blink === 0 && <span className="face-glint" />}
      </div>
      <div className="face-eye" style={{ right: 20 * s, top: 30 * s, ...merged }}>
        {glint && mood !== 'happy' && blink === 0 && <span className="face-glint" />}
      </div>
    </div>
  );
}

/** 内嵌弥散极光层（磨砂玻璃球的"内心"） */
function AuroraCore() {
  return (
    <div className="aurora-core">
      <span className="blob b1" />
      <span className="blob b2" />
      <span className="blob b3" />
      <span className="blob b4" />
    </div>
  );
}

export function Orb({
  persona, setPersona, apiKey, setApiKey, onOpenChat, chatOpen,
}: {
  persona: Persona; setPersona: (p: Persona) => void;
  apiKey: string; setApiKey: (k: string) => void;
  onOpenChat: () => void; chatOpen: boolean;
}) {
  const orbRef = useRef<HTMLDivElement>(null);
  const look = useLook(orbRef);
  const blink = useBlink();
  const [mood, setMood] = useState<Mood>('normal');
  const [hover, setHover] = useState(false);
  // 位置初始化：消毒 localStorage（防拖到屏幕外后启动看不见球）
  const [pos, setPos] = useState(() => {
    const fallback = { x: window.innerWidth - 120, y: window.innerHeight - 220 };
    try {
      const s = localStorage.getItem('zrsj_orb_pos');
      if (!s) return fallback;
      const p = JSON.parse(s);
      const ok = Number.isFinite(p.x) && Number.isFinite(p.y)
        && p.x >= -20 && p.x <= window.innerWidth - 40
        && p.y >= -20 && p.y <= window.innerHeight - 40;
      return ok ? p : fallback;
    } catch {
      return fallback;
    }
  });
  const [menu, setMenu] = useState(false);
  const [colorName, setColorName] = useState(localStorage.getItem('zrsj_orb_color') ?? '星紫');
  const [bubble, setBubble] = useState(BUBBLES[0]);
  const dragRef = useRef<{ dx: number; dy: number; moved: boolean } | null>(null);
  const draggedRef = useRef(false); // 用户拖动过 → 尊重自定义位

  // 窗口尺寸自适应：未拖动时锚定右下角；拖动过则夹紧进可视区
  useEffect(() => {
    if (localStorage.getItem('zrsj_orb_pos')) draggedRef.current = true; // 存档位视为自定义
    const onResize = () => {
      if (!draggedRef.current) {
        setPos({ x: Math.max(0, window.innerWidth - 120), y: Math.max(0, window.innerHeight - 220) });
      } else {
        setPos((p: { x: number; y: number }) => ({
          x: Math.max(0, Math.min(p.x, window.innerWidth - 40)),
          y: Math.max(0, Math.min(p.y, window.innerHeight - 40)),
        }));
      }
    };
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  // 表情状态机：悬停好奇 · 鼠标上方抬头时 wide · 平时偶尔 think · 点击后 happy
  useEffect(() => {
    if (hover) {
      setMood(look.pitch > 5 || look.pitch < -5 ? 'wide' : 'curious');
      setBubble(BUBBLES[Math.floor(Math.random() * BUBBLES.length)]);
    } else {
      setMood('normal');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hover, look.pitch]);

  useEffect(() => {
    const t = setInterval(() => {
      setMood((m) => (m === 'normal' ? 'think' : m === 'think' ? 'normal' : m));
    }, 7000);
    return () => clearInterval(t);
  }, []);

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      if (!dragRef.current) return;
      const nx = e.clientX - dragRef.current.dx;
      const ny = e.clientY - dragRef.current.dy;
      if (Math.abs(nx - pos.x) + Math.abs(ny - pos.y) > 4) dragRef.current.moved = true;
      setPos({ x: Math.max(0, Math.min(window.innerWidth - 88, nx)), y: Math.max(0, Math.min(window.innerHeight - 88, ny)) });
    };
    const onUp = () => {
      if (dragRef.current) {
        const wasDrag = dragRef.current.moved;
        dragRef.current = null;
        if (wasDrag) draggedRef.current = true;
        setPos((p: { x: number; y: number }) => {
          localStorage.setItem('zrsj_orb_pos', JSON.stringify(p));
          return p;
        });
        if (!wasDrag) {
          setMood('happy');
          setTimeout(() => setMood('normal'), 700);
          onOpenChat();
        }
      }
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pos]);

  const c = COLORS[colorName] ?? COLORS['虹彩'];

  return (
    <>
      {hover && !chatOpen && (
        <div className="orb-bubble" style={{ left: Math.max(8, pos.x - 168), top: pos.y + 10 }}>
          <span className="bubble-text">{bubble}</span>
          <span className="bubble-tail" />
        </div>
      )}
      <div
        ref={orbRef}
        className={`orb-float ${chatOpen ? 'chatting' : ''} ${hover ? 'hover' : ''}`}
        style={{
          left: pos.x,
          top: pos.y,
          boxShadow: `0 14px 36px ${c.glow}`,
          transform: `scale(${hover ? 1.05 : 1})`,
        }}
        onMouseDown={(e) => {
          dragRef.current = { dx: e.clientX - pos.x, dy: e.clientY - pos.y, moved: false };
          setMenu(false);
        }}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        onContextMenu={(e) => {
          e.preventDefault();
          setMenu(true);
        }}
        title="数字财务 · 点击对话 · 拖拽移动 · 右键设置"
      >
        <div className="aurora-wrap" style={{ filter: `hue-rotate(${c.hue}deg)` }}>
          <AuroraCore />
        </div>
        <div className="orb-glass" />
        <JellyFace look={look} blink={blink} mood={mood} glint={false} />
      </div>

      {menu && (
        <div className="orb-menu" style={{ left: Math.min(pos.x + 96, window.innerWidth - 280), top: pos.y }}>
          <div className="orb-menu-title">数字财务 · 设置</div>
          <label>性格</label>
          <div className="seg">
            <button className={persona === 'gentle' ? 'on' : ''} onClick={() => setPersona('gentle')}>温柔</button>
            <button className={persona === 'boss' ? 'on' : ''} onClick={() => setPersona('boss')}>霸总</button>
          </div>
          <label>颜色（高级感）</label>
          <div className="swatches">
            {Object.keys(COLORS).map((k) => (
              <button
                key={k}
                className={colorName === k ? 'sw on' : 'sw'}
                style={{ background: 'linear-gradient(135deg,#c4b5f8,#7dd3fc,#f9a8d4)', filter: `hue-rotate(${COLORS[k].hue}deg)` }}
                onClick={() => {
                  setColorName(k);
                  localStorage.setItem('zrsj_orb_color', k);
                }}
              />
            ))}
          </div>
          <label>API Key（自带 Key，直达模型商）</label>
          <input
            type="password"
            placeholder="sk-..."
            value={apiKey}
            onChange={(e) => {
              setApiKey(e.target.value);
              localStorage.setItem('zrsj_ai_key', e.target.value);
            }}
          />
        </div>
      )}
    </>
  );
}

/** 对话卡头部：果冻公仔（同款表情系统，持续 Q 弹） */
export function Mascot() {
  const ref = useRef<HTMLDivElement>(null);
  const look = useLook(ref);
  const blink = useBlink();
  const [mood, setMood] = useState<Mood>('normal');
  useEffect(() => {
    const t = setInterval(() => {
      setMood((m) => (m === 'normal' ? 'think' : m === 'think' ? 'happy' : 'normal'));
    }, 5200);
    return () => clearInterval(t);
  }, []);
  return (
    <div className="mascot" ref={ref}>
      <div className="mascot-jelly">
        <AuroraCore />
        <div className="mascot-gloss" />
        <JellyFace look={look} blink={blink} mood={mood} size={96} />
      </div>
    </div>
  );
}
