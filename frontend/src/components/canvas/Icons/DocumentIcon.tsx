import { motion } from 'framer-motion';

export function DocumenterIcon() {
  // 8-tooth gear polygon centered at origin, scaled down (~0.85x): inner ~10, outer ~13.5
  const gearPoints =
    '-1.6,-10.1 -1.7,-13.5 1.7,-13.5 1.6,-10.1 ' +
    '6.0,-8.2 8.4,-10.7 10.7,-8.3 8.2,-6.0 ' +
    '10.1,-1.6 13.5,-1.6 13.5,1.7 10.1,1.6 ' +
    '8.2,6.0 10.7,8.4 8.4,10.7 6.0,8.2 ' +
    '1.6,10.1 1.7,13.5 -1.7,13.5 -1.6,10.1 ' +
    '-6.0,8.2 -8.4,10.7 -10.7,8.4 -8.2,6.0 ' +
    '-10.1,1.6 -13.5,1.7 -13.5,-1.6 -10.1,-1.6 ' +
    '-8.2,-6.0 -10.7,-8.3 -8.4,-10.7 -6.0,-8.2';

  return (
    <svg
      width="36"
      height="36"
      viewBox="0 0 36 36"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Spinning gear behind the document — drawn at origin, translated to center */}
      <g transform="translate(18, 18)">
        <motion.g
          animate={{ rotate: 360 }}
          transition={{ duration: 20, repeat: Infinity, ease: 'linear' }}
        >
          <polygon
            points={gearPoints}
            fill="#E8862E"
            opacity="0.85"
            stroke="#E8862E"
            strokeWidth="1.2"
            strokeLinejoin="round"
          />
        </motion.g>
      </g>
      {/* Gear center ring */}
      <circle cx="18" cy="18" r="4" fill="none" stroke="#E8862E" strokeWidth="1.5" opacity="0.6" />

      {/* Document sitting on top — dark stroke for contrast against orange gear */}
      <rect x="11" y="9" width="14" height="18" rx="2" fill="white" stroke="#78909C" strokeWidth="1" />
      {/* Corner fold */}
      <path d="M21 9v3.5a1.5 1.5 0 001.5 1.5H26" stroke="#78909C" strokeWidth="0.8" fill="white" />
      {/* Text lines */}
      <rect x="13.5" y="17" width="9" height="1.2" rx="0.6" fill="#78909C" opacity="0.5" />
      <rect x="13.5" y="19.5" width="7" height="1.2" rx="0.6" fill="#78909C" opacity="0.3" />
      <rect x="13.5" y="22" width="9" height="1.2" rx="0.6" fill="#78909C" opacity="0.2" />
    </svg>
  );
}

export function HeroIllustration() {
  return (
    <div className="w-full max-w-lg mx-auto">
      <svg viewBox="0 0 400 320" fill="none" xmlns="http://www.w3.org/2000/svg">
        {/* Document shape */}
        <motion.rect
          initial={{ opacity: 0, scale: 0.9 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.6 }}
          style={{ transformBox: 'fill-box', transformOrigin: 'center' }}
          x="80"
          y="40"
          width="160"
          height="200"
          rx="8"
          fill="white"
          stroke="#E5E7EB"
          strokeWidth="2"
        />

        {/* Document text lines */}
        {[0, 1, 2, 3, 4].map((i) => (
          <motion.rect
            key={`line-${i}`}
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ duration: 0.4, delay: 0.3 + i * 0.1 }}
            x="104"
            y={76 + i * 28}
            width={i === 4 ? 80 : 112}
            height="8"
            rx="4"
            fill={i < 2 ? '#C4854A' : '#E5E7EB'}
            opacity={i < 2 ? 0.3 : 0.5}
          />
        ))}

        {/* Document corner fold */}
        <motion.path
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.4, delay: 0.2 }}
          d="M216 40v24a8 8 0 008 8h16"
          stroke="#E5E7EB"
          strokeWidth="2"
          fill="#F9FAFB"
        />

        {/* Connection lines from document to gear */}
        <motion.path
          initial={{ pathLength: 0, opacity: 0 }}
          animate={{ pathLength: 1, opacity: 1 }}
          transition={{ duration: 0.8, delay: 0.8 }}
          d="M240 140h40q10 0 10 10v20"
          stroke="#C4854A"
          strokeWidth="2"
          strokeDasharray="6 4"
          fill="none"
        />

        {/* Connection line to bottom node */}
        <motion.path
          initial={{ pathLength: 0, opacity: 0 }}
          animate={{ pathLength: 1, opacity: 1 }}
          transition={{ duration: 0.8, delay: 1.0 }}
          d="M290 210v30q0 10 10 10h20"
          stroke="#C4854A"
          strokeWidth="2"
          strokeDasharray="6 4"
          fill="none"
        />

        {/* Gear shape */}
        <motion.g
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 0.6, delay: 1.0 }}
        >
          <circle cx="290" cy="190" r="24" fill="#FFF7ED" stroke="#C4854A" strokeWidth="2" />
          <animateTransform
            xlinkHref="#gear-teeth"
            attributeName="transform"
            type="rotate"
            from="0 290 190"
            to="360 290 190"
            dur="20s"
            repeatCount="indefinite"
          />
          <path
            id="gear-teeth"
            d="M290 162a4 4 0 014 4v4a20 20 0 018.5 4.9l2.8-2.8a4 4 0 015.7 5.7l-2.8 2.8A20 20 0 01314 190h4a4 4 0 010 8h-4a20 20 0 01-4.9 8.5l2.8 2.8a4 4 0 01-5.7 5.7l-2.8-2.8a20 20 0 01-8.5 4.9v4a4 4 0 01-8 0v-4a20 20 0 01-8.5-4.9l-2.8 2.8a4 4 0 01-5.7-5.7l2.8-2.8A20 20 0 01266 198h-4a4 4 0 010-8h4a20 20 0 014.9-8.5l-2.8-2.8a4 4 0 015.7-5.7l2.8 2.8A20 20 0 01286 170v-4a4 4 0 014-4z"
            fill="none"
            stroke="#C4854A"
            strokeWidth="1.5"
            opacity="0.4"
          />
          <circle cx="290" cy="190" r="10" fill="none" stroke="#C4854A" strokeWidth="2" />
        </motion.g>

        {/* Floating data nodes */}
        <motion.circle
          animate={{ y: [0, -6, 0] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
          cx="340"
          cy="120"
          r="12"
          fill="#FFF7ED"
          stroke="#C4854A"
          strokeWidth="2"
        />
        <motion.rect
          animate={{ y: [0, -6, 0] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut', delay: 0.5 }}
          x="310"
          y="238"
          width="40"
          height="24"
          rx="6"
          fill="#FFF7ED"
          stroke="#C4854A"
          strokeWidth="2"
        />

        {/* Small check mark inside bottom node */}
        <motion.path
          animate={{ y: [0, -6, 0] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut', delay: 0.5 }}
          d="M323 250l5 5 9-9"
          stroke="#C4854A"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Small dots in top node */}
        <motion.g animate={{ y: [0, -6, 0] }} transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}>
          <circle cx="336" cy="118" r="2" fill="#C4854A" />
          <circle cx="340" cy="122" r="2" fill="#C4854A" />
          <circle cx="344" cy="118" r="2" fill="#C4854A" />
        </motion.g>

        {/* Pulse ring around gear */}
        <motion.circle
          animate={{ scale: [1, 1.4, 1], opacity: [0.3, 0, 0.3] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
          style={{ transformBox: 'fill-box', transformOrigin: 'center' }}
          cx="290"
          cy="190"
          r="30"
          fill="none"
          stroke="#C4854A"
          strokeWidth="1"
        />

        {/* Circuit dots */}
        <motion.circle
          initial={{ opacity: 0 }}
          animate={{ opacity: [0, 1, 0] }}
          transition={{ duration: 2, repeat: Infinity, delay: 0.5 }}
          cx="260"
          cy="140"
          r="3"
          fill="#C4854A"
        />
        <motion.circle
          initial={{ opacity: 0 }}
          animate={{ opacity: [0, 1, 0] }}
          transition={{ duration: 2, repeat: Infinity, delay: 1.0 }}
          cx="290"
          cy="220"
          r="3"
          fill="#C4854A"
        />
      </svg>
    </div>
  );
}