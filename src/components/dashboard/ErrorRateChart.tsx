import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { useEngineStore } from "@/stores/useEngineStore";

export function ErrorRateChart() {
  const chartData = useEngineStore((s) => s.chartData);

  if (chartData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-sm text-muted-foreground">
        Waiting for data...
      </div>
    );
  }

  return (
    <ResponsiveContainer width="100%" height={280}>
      <AreaChart
        data={chartData}
        margin={{ top: 5, right: 20, left: 10, bottom: 5 }}
      >
        <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
        <XAxis
          dataKey="elapsed_s"
          tick={{ fontSize: 11 }}
          label={{
            value: "Time (s)",
            position: "insideBottom",
            offset: -5,
            fontSize: 11,
          }}
        />
        <YAxis
          tick={{ fontSize: 11 }}
          domain={[0, "auto"]}
          label={{
            value: "%",
            angle: -90,
            position: "insideLeft",
            fontSize: 11,
          }}
        />
        <Tooltip
          contentStyle={{ fontSize: 12 }}
          formatter={(value: number) => [
            `${value.toFixed(1)}%`,
            "Error Rate",
          ]}
        />
        <Area
          type="monotone"
          dataKey="error_rate"
          name="Error Rate"
          stroke="#ef4444"
          fill="#ef4444"
          fillOpacity={0.15}
          strokeWidth={2}
          dot={false}
        />
      </AreaChart>
    </ResponsiveContainer>
  );
}
