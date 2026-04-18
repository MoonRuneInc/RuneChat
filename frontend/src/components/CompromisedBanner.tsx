interface Props {
  username: string
}

export default function CompromisedBanner({ username }: Props) {
  return (
    <div className="bg-amber-900/30 border border-amber-600/40 text-amber-300 text-sm px-4 py-2.5 rounded-lg">
      <span className="font-semibold">{username}</span>'s account has been flagged as potentially compromised.
      Messages sent after the compromise event are marked.
    </div>
  )
}

export function CompromisedBadge() {
  return (
    <span
      className="inline-flex items-center px-1.5 py-0.5 rounded text-xs bg-amber-900/50 text-amber-400 border border-amber-700/50 ml-1"
      title="Account flagged as compromised"
    >
      ⚠ compromised
    </span>
  )
}
