import { motion } from 'framer-motion';

export function DocumenterIcon() {
  // 8-tooth gear polygon centered at origin (0,0), inner radius 12, outer radius 16
  const gearPoints =
    '-1.9,-11.9 -2.0,-15.9 2.0,-15.9 1.9,-11.9 ' +
    '7.1,-9.7 9.9,-12.6 12.6,-9.8 9.7,-7.0 ' +
    '11.9,-1.9 15.9,-1.9 15.9,2.0 11.9,1.9 ' +
    '9.7,7.1 12.6,9.9 9.9,12.6 7.1,9.7 ' +
    '1.9,11.9 2.0,15.9 -2.0,15.9 -1.9,11.9 ' +
    '-7.0,9.7 -9.9,12.6 -12.6,9.9 -9.7,7.1 ' +
    '-11.9,1.9 -15.9,2.0 -15.9,-1.9 -11.9,-1.9 ' +
    '-9.7,-7.0 -12.6,-9.8 -9.9,-12.6 -7.0,-9.7';

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
            fill="#F57C00"
            opacity="0.85"
            stroke="#F57C00"
            strokeWidth="1.2"
            strokeLinejoin="round"
          />
        </motion.g>
      </g>
      {/* Gear center ring */}
      <circle cx="18" cy="18" r="5" fill="none" stroke="#F57C00" strokeWidth="1.5" opacity="0.6" />

      {/* Document sitting on top */}
      <rect x="10" y="8" width="16" height="20" rx="2" fill="white" stroke="#F57C00" strokeWidth="1.5" />
      {/* Corner fold */}
      <path d="M22 8v4a2 2 0 002 2h2" stroke="#F57C00" strokeWidth="1.2" fill="white" />
      {/* Text lines */}
      <rect x="13" y="16" width="10" height="1.2" rx="0.6" fill="#F57C00" opacity="0.5" />
      <rect x="13" y="19" width="8" height="1.2" rx="0.6" fill="#F57C00" opacity="0.3" />
      <rect x="13" y="22" width="10" height="1.2" rx="0.6" fill="#F57C00" opacity="0.2" />
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
            fill={i < 2 ? '#F57C00' : '#E5E7EB'}
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
          stroke="#F57C00"
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
          stroke="#F57C00"
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
          <circle cx="290" cy="190" r="24" fill="#FFF7ED" stroke="#F57C00" strokeWidth="2" />
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
            stroke="#F57C00"
            strokeWidth="1.5"
            opacity="0.4"
          />
          <circle cx="290" cy="190" r="10" fill="none" stroke="#F57C00" strokeWidth="2" />
        </motion.g>

        {/* Floating data nodes */}
        <motion.circle
          animate={{ y: [0, -6, 0] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}
          cx="340"
          cy="120"
          r="12"
          fill="#FFF7ED"
          stroke="#F57C00"
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
          stroke="#F57C00"
          strokeWidth="2"
        />

        {/* Small check mark inside bottom node */}
        <motion.path
          animate={{ y: [0, -6, 0] }}
          transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut', delay: 0.5 }}
          d="M323 250l5 5 9-9"
          stroke="#F57C00"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />

        {/* Small dots in top node */}
        <motion.g animate={{ y: [0, -6, 0] }} transition={{ duration: 3, repeat: Infinity, ease: 'easeInOut' }}>
          <circle cx="336" cy="118" r="2" fill="#F57C00" />
          <circle cx="340" cy="122" r="2" fill="#F57C00" />
          <circle cx="344" cy="118" r="2" fill="#F57C00" />
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
          stroke="#F57C00"
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
          fill="#F57C00"
        />
        <motion.circle
          initial={{ opacity: 0 }}
          animate={{ opacity: [0, 1, 0] }}
          transition={{ duration: 2, repeat: Infinity, delay: 1.0 }}
          cx="290"
          cy="220"
          r="3"
          fill="#F57C00"
        />
      </svg>
    </div>
  );
}