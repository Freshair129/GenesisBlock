

interface Props {
  totalAtoms: number
  inboundCount: number
  hotfixCount: number
}

export default function StatusBar({ totalAtoms, inboundCount, hotfixCount }: Props) {
  return (
    <div className="status-bar">
      <span>{totalAtoms} atoms</span>
      <span>{hotfixCount} hotfixes</span>
      <span>{inboundCount} in inbound</span>
    </div>
  )
}
