import { useEffect, useRef, useState, type RefObject } from 'react'

type BenchmarkMetric = {
  label: string
  detail: string
  olderLabel: string
  v2Label: string
  reductionLabel: string
  changeLabel: string
  v2Share: number
}

type InstructionMetric = BenchmarkMetric & {
  older: number
  v2: number
  reduction: number
}

type ProgramBenchmark = {
  id: string
  label: string
  detail: string
  binary: BenchmarkMetric
  compute: BenchmarkMetric
}

type ActiveTab = 'binary' | 'compute'

const programs: ProgramBenchmark[] = [
  programBenchmark({
    id: 'helloworld',
    label: 'helloworld',
    detail: 'Single-instruction counter',
    binary: [124_624, 6_440],
    instructions: [{ label: 'init', detail: 'Counter initialization', older: 5_855, v2: 1_381 }],
  }),
  programBenchmark({
    id: 'prop-amm',
    label: 'prop-amm',
    detail: 'Oracle feed with asm fast-path',
    binary: [140_280, 8_592],
    instructions: [
      { label: 'initialize', detail: 'Oracle setup', older: 4_314, v2: 1_375 },
      { label: 'rotate_authority', detail: 'Authority rotation', older: 1_340, v2: 84 },
      { label: 'update', detail: 'Asm fast path', older: 1_310, v2: 26 },
    ],
  }),
  programBenchmark({
    id: 'vault',
    label: 'vault',
    detail: 'Single-depositor SOL vault',
    binary: [107_368, 5_384],
    instructions: [
      { label: 'deposit', detail: 'System transfer CPI', older: 5_707, v2: 1_899 },
      { label: 'withdraw', detail: 'Lamport withdrawal', older: 2_478, v2: 389 },
    ],
  }),
  programBenchmark({
    id: 'nested',
    label: 'nested',
    detail: 'Shared validation via Nested<T>',
    binary: [157_160, 12_424],
    instructions: [
      { label: 'initialize', detail: 'Shared validation setup', older: 19_842, v2: 2_716 },
      { label: 'increment', detail: 'Nested validation path', older: 4_751, v2: 474 },
      { label: 'reset', detail: 'Nested reset path', older: 4_752, v2: 473 },
    ],
  }),
  programBenchmark({
    id: 'multisig',
    label: 'multisig',
    detail: 'Four-instruction SOL multisig',
    binary: [169_920, 30_976],
    instructions: [
      { label: 'create', detail: 'Config creation', older: 12_031, v2: 3_016 },
      { label: 'deposit', detail: 'Vault funding', older: 4_872, v2: 1_613 },
      { label: 'set_label', detail: 'Inline PodVec update', older: 4_324, v2: 469 },
      { label: 'execute_transfer', detail: 'Threshold transfer', older: 7_446, v2: 2_170 },
    ],
  }),
]

const tabs = [
  { value: 'binary', label: 'Binary size' },
  { value: 'compute', label: 'Compute units' },
] satisfies Array<{ value: ActiveTab; label: string }>

const proofOldColor = 'bg-[color-mix(in_oklch,var(--ctp-overlay-0)_76%,transparent)]'
const proofOldColorDark = 'dark:bg-[color-mix(in_oklch,var(--ctp-overlay-0)_68%,transparent)]'
const tabularNums = '[font-feature-settings:"tnum"_1] [font-variant-numeric:tabular-nums]'

function programBenchmark({
  id,
  label,
  detail,
  binary,
  instructions,
}: {
  id: string
  label: string
  detail: string
  binary: [older: number, v2: number]
  instructions: Array<{ label: string; detail: string; older: number; v2: number }>
}): ProgramBenchmark {
  const instructionMetrics = instructions.map((instruction) =>
    instructionMetric(instruction.label, instruction.detail, instruction.older, instruction.v2),
  )
  const leastImproved = instructionMetrics.reduce((current, next) =>
    next.v2Share > current.v2Share ? next : current,
  )

  return {
    id,
    label,
    detail,
    binary: singleMetric(label, detail, binary[0], binary[1], 'bytes', 'smaller'),
    compute: {
      label,
      detail,
      olderLabel: formatCuRange(instructionMetrics.map((instruction) => instruction.older)),
      v2Label: formatCuRange(instructionMetrics.map((instruction) => instruction.v2)),
      reductionLabel: formatReductionRange(
        instructionMetrics.map((instruction) => instruction.reduction),
      ),
      changeLabel: 'lower CU',
      v2Share: leastImproved.v2Share,
    },
  }
}

function singleMetric(
  label: string,
  detail: string,
  older: number,
  v2: number,
  unit: 'bytes' | 'cu',
  changeLabel: string,
): BenchmarkMetric {
  const reduction = older / v2

  return {
    label,
    detail,
    olderLabel: unit === 'bytes' ? formatKb(older) : `${formatNumber(older)} CU`,
    v2Label: unit === 'bytes' ? formatKb(v2) : `${formatNumber(v2)} CU`,
    reductionLabel: `${formatReduction(reduction)}x`,
    changeLabel,
    v2Share: (v2 / older) * 100,
  }
}

function instructionMetric(
  label: string,
  detail: string,
  older: number,
  v2: number,
): InstructionMetric {
  const reduction = older / v2

  return {
    label,
    detail,
    older,
    v2,
    olderLabel: `${formatNumber(older)} CU`,
    v2Label: `${formatNumber(v2)} CU`,
    reduction,
    reductionLabel: `${formatReduction(reduction)}x`,
    changeLabel: 'lower CU',
    v2Share: (v2 / older) * 100,
  }
}

function formatKb(bytes: number) {
  const kb = bytes / 1000
  return `${kb >= 10 ? kb.toFixed(1).replace('.0', '') : kb.toFixed(1)} KB`
}

function formatNumber(value: number) {
  return new Intl.NumberFormat('en-US').format(value)
}

function formatReduction(value: number) {
  return value >= 10 ? value.toFixed(1).replace('.0', '') : value.toFixed(1)
}

function formatCuRange(values: number[]) {
  const min = Math.min(...values)
  const max = Math.max(...values)

  if (min === max) return `${formatNumber(min)} CU`

  return `${formatNumber(min)}-${formatNumber(max)} CU`
}

function formatReductionRange(values: number[]) {
  const min = Math.min(...values)
  const max = Math.max(...values)

  if (min === max) return `${formatReduction(min)}x`

  return `${formatReduction(min)}-${formatReduction(max)}x`
}

function BenchmarkMeter({ row }: { row: BenchmarkMetric }) {
  return (
    <div className="min-w-0 self-end [grid-area:meter]">
      <div
        className={`${proofOldColor} ${proofOldColorDark} relative h-3 min-w-0 overflow-hidden rounded-full`}
        aria-hidden="true"
      >
        <span
          className="bg-accent absolute inset-y-0 left-0 min-w-2.5 rounded-[inherit]"
          style={{ width: `${Math.max(row.v2Share, 1.5)}%` }}
        />
      </div>
      <div
        className={`text-muted-foreground mt-3 grid min-w-0 grid-cols-2 gap-2 text-[0.8125rem] leading-[1.2] ${tabularNums}`}
      >
        <span className="flex min-w-0 flex-col gap-0.5">
          <span className="text-muted-foreground/75 text-[0.6875rem] leading-none">v1</span>
          {row.olderLabel}
        </span>
        <span className="flex min-w-0 flex-col gap-0.5">
          <span className="text-muted-foreground/75 text-[0.6875rem] leading-none">v2</span>
          {row.v2Label}
        </span>
      </div>
    </div>
  )
}

function ReductionMetric({ row }: { row: BenchmarkMetric }) {
  return (
    <div className="min-w-0 text-right [grid-area:reduction]">
      <div
        className={`text-foreground text-2xl leading-none font-medium whitespace-nowrap ${tabularNums}`}
      >
        {row.reductionLabel}
      </div>
      <div className="text-muted-foreground mt-1.5 text-[0.8125rem] leading-[1.1] whitespace-nowrap">
        {row.changeLabel}
      </div>
    </div>
  )
}

function ProgramSummary({ row }: { row: BenchmarkMetric }) {
  return (
    <div className="grid w-full min-w-0 grid-cols-[minmax(0,1fr)_max-content] grid-rows-[auto_1fr] gap-x-4 gap-y-5 [grid-template-areas:'text_reduction'_'meter_meter']">
      <div className="min-w-0 [grid-area:text]">
        <div className="text-foreground overflow-hidden text-base leading-[1.2] font-medium text-ellipsis whitespace-nowrap">
          {row.label}
        </div>
        <div className="text-muted-foreground mt-1.5 overflow-hidden text-sm leading-[1.3] [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:2]">
          {row.detail}
        </div>
      </div>

      <BenchmarkMeter row={row} />

      <ReductionMetric row={row} />
    </div>
  )
}

function ProgramRow({ program, activeTab }: { program: ProgramBenchmark; activeTab: ActiveTab }) {
  const row = activeTab === 'binary' ? program.binary : program.compute

  return (
    <div
      className="border-border/80 bg-background flex min-h-[11.5rem] min-w-0 flex-[0_0_min(24rem,calc(100vw-2rem))] snap-start rounded-[1.35rem] border-2 p-[1.125rem] sm:min-h-[12.25rem] sm:flex-basis-[25.5rem] lg:flex-basis-[27rem] xl:flex-basis-[25.5rem]"
      data-proof-row
    >
      <ProgramSummary row={row} />
    </div>
  )
}

function BenchmarkPanel({
  activeTab,
  scrollRef,
}: {
  activeTab: ActiveTab
  scrollRef: RefObject<HTMLDivElement | null>
}) {
  return (
    <div
      id={`proof-${activeTab}-panel`}
      role="tabpanel"
      aria-labelledby={`proof-${activeTab}-tab`}
      className="mt-3 w-full"
    >
      <div
        className="relative mt-3 -mx-2 w-[calc(100%+1rem)] before:pointer-events-none before:absolute before:top-0 before:bottom-3 before:left-0 before:z-10 before:w-10 before:bg-gradient-to-r before:from-background before:to-transparent before:opacity-0 before:transition-opacity after:pointer-events-none after:absolute after:top-0 after:right-0 after:bottom-3 after:z-10 after:w-10 after:bg-gradient-to-l after:from-background after:to-transparent after:opacity-0 after:transition-opacity data-[left-fade=true]:before:opacity-100 data-[right-fade=true]:after:opacity-100 lg:-mx-4 lg:w-[calc(100%+2rem)] lg:before:w-[3.25rem] lg:after:w-[3.25rem]"
        data-proof-scroll-frame
      >
        <div
          ref={scrollRef}
          className="flex w-full snap-x snap-mandatory items-stretch gap-3 overflow-x-auto overscroll-x-contain scroll-px-2 px-2 pb-3 [scrollbar-width:none] lg:gap-4 lg:scroll-px-4 lg:px-4 [&::-webkit-scrollbar]:hidden"
        >
          {programs.map((program) => (
            <ProgramRow key={program.id} program={program} activeTab={activeTab} />
          ))}
        </div>
      </div>
    </div>
  )
}

function Stat({ value, label }: { value: string; label: string }) {
  return (
    <div className="border-border/80 min-w-0 rounded-2xl border-2 p-4">
      <div
        className={`text-foreground text-2xl leading-none font-medium sm:text-3xl ${tabularNums}`}
      >
        {value}
      </div>
      <div className="text-muted-foreground mt-2 text-[0.8125rem] leading-[1.3]">{label}</div>
    </div>
  )
}

export default function LandingProofChart() {
  const [activeTab, setActiveTab] = useState<ActiveTab>('binary')
  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const scrollArea = scrollRef.current
    if (!scrollArea) return
    const scrollFrame = scrollArea.closest<HTMLElement>('[data-proof-scroll-frame]')
    if (!scrollFrame) return

    let animationFrame = 0

    const updateFade = () => {
      animationFrame = 0
      const { scrollLeft, scrollWidth, clientWidth } = scrollArea
      const threshold = 2
      const overflow = scrollWidth - clientWidth
      const isAtStart = scrollLeft <= threshold
      const isAtEnd = overflow <= threshold || scrollLeft >= overflow - threshold
      const scrollAreaStyles = window.getComputedStyle(scrollArea)
      const scrollAreaRect = scrollArea.getBoundingClientRect()
      const leftEdge = scrollAreaRect.left + Number.parseFloat(scrollAreaStyles.paddingLeft)
      const rightEdge = scrollAreaRect.right - Number.parseFloat(scrollAreaStyles.paddingRight)
      const snapThreshold = 3
      const rows = Array.from(scrollArea.querySelectorAll<HTMLElement>('[data-proof-row]'))
      const hasSnappedLeftRow = rows.some(
        (row) => Math.abs(row.getBoundingClientRect().left - leftEdge) <= snapThreshold,
      )
      const hasSnappedRightRow = rows.some(
        (row) => Math.abs(row.getBoundingClientRect().right - rightEdge) <= snapThreshold,
      )

      scrollFrame.dataset.leftFade = !isAtStart && !hasSnappedLeftRow ? 'true' : 'false'
      scrollFrame.dataset.rightFade = !isAtEnd && !hasSnappedRightRow ? 'true' : 'false'
    }

    const scheduleFadeUpdate = () => {
      if (animationFrame) return
      animationFrame = window.requestAnimationFrame(updateFade)
    }

    scrollArea.scrollLeft = 0
    updateFade()
    scrollArea.addEventListener('scroll', scheduleFadeUpdate, { passive: true })
    window.addEventListener('resize', scheduleFadeUpdate)

    return () => {
      if (animationFrame) window.cancelAnimationFrame(animationFrame)
      scrollArea.removeEventListener('scroll', scheduleFadeUpdate)
      window.removeEventListener('resize', scheduleFadeUpdate)
    }
  }, [activeTab])

  return (
    <section
      id="proof-in-numbers"
      className="not-prose mt-20 scroll-mt-[calc(var(--docs-announcement-offset,0px)+5rem)]"
      aria-labelledby="proof-in-numbers-title"
    >
      <div className="flex items-end justify-between gap-4">
        <div className="max-w-2xl">
          <h2
            id="proof-in-numbers-title"
            className="text-foreground m-0 text-2xl leading-[1.1] font-medium text-balance"
          >
            Anchor v2 in practice
          </h2>
          <p className="text-muted-foreground mt-2 mb-0 text-base leading-[1.45] text-pretty">
            Anchor v2 keeps the workflow developers rely on while making the generated program
            smaller, lighter, and cheaper to run.
          </p>
        </div>
      </div>

      <div className="mt-4 grid grid-cols-3 gap-2">
        <Stat value="95%" label="less deployed bytecode" />
        <Stat value="10x" label="average CU reduction" />
        <Stat value="50.4x" label="largest CU reduction" />
      </div>

      <div className="mt-4 flex w-full flex-col items-stretch">
        <div className="flex w-full flex-wrap items-center justify-between gap-x-4 gap-y-3">
          <div
            role="tablist"
            aria-label="Benchmark metric"
            className="border-border/80 bg-muted/30 inline-flex h-[2.875rem] max-w-full items-center gap-1 overflow-x-auto rounded-md border-2 p-1 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
          >
            {tabs.map((tab) => (
              <button
                key={tab.value}
                id={`proof-${tab.value}-tab`}
                type="button"
                role="tab"
                aria-selected={activeTab === tab.value}
                aria-controls={`proof-${tab.value}-panel`}
                className="text-muted-foreground hover:bg-muted/60 hover:text-foreground aria-selected:bg-background aria-selected:text-foreground h-[calc(2.875rem-0.5rem-4px)] flex-none cursor-pointer rounded-sm border-0 bg-transparent px-4 text-base leading-none font-medium whitespace-nowrap transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
                onClick={() => setActiveTab(tab.value)}
              >
                {tab.label}
              </button>
            ))}
          </div>

          <div
            className="border-border/80 bg-background text-muted-foreground inline-grid h-[2.875rem] content-center gap-1 rounded-md border-2 px-2.5 py-1.5 text-[0.6875rem] leading-none font-medium"
            aria-label="Benchmark comparison key"
          >
            <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
              <span
                className={`${proofOldColor} ${proofOldColorDark} h-[0.3125rem] w-[1.125rem] flex-none rounded-full`}
                aria-hidden="true"
              />
              v1 baseline
            </span>
            <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
              <span
                className="bg-accent h-[0.3125rem] w-[1.125rem] flex-none rounded-full"
                aria-hidden="true"
              />
              v2 runtime
            </span>
          </div>
        </div>

        <BenchmarkPanel activeTab={activeTab} scrollRef={scrollRef} />
      </div>
    </section>
  )
}
