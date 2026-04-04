defmodule Translator.MixProject do
  use Mix.Project

  @native_dir Path.join([__DIR__, "native", "audio_engine"])

  def project do
    [
      app: :translator,
      version: "0.1.0",
      elixir: "~> 1.19",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      compilers: [:rust] ++ Mix.compilers(),
      aliases: aliases()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Translator.Application, []}
    ]
  end

  defp deps do
    [
      {:jason, "1.4.4"}
    ]
  end

  defp aliases do
    [
      "compile.rust": &compile_rust/1
    ]
  end

  defp compile_rust(_args) do
    native_dir = @native_dir

    if File.dir?(native_dir) do
      home = System.get_env("HOME") || ""
      system_path = System.get_env("PATH") || ""
      path = "#{home}/.cargo/bin:/opt/homebrew/opt/rustup/bin:#{system_path}"

      cargo =
        Enum.find_value(String.split(path, ":"), fn dir ->
          full = Path.join(dir, "cargo")
          if File.exists?(full), do: full
        end) || "cargo"

      IO.puts("Compiling Rust audio_engine...")

      case System.cmd(cargo, ["build", "--release"],
             cd: native_dir,
             stderr_to_stdout: true,
             env: [{"PATH", path}, {"MACOSX_DEPLOYMENT_TARGET", "14.0"}]
           ) do
        {output, 0} ->
          IO.puts(output)
          IO.puts("Rust audio_engine compiled successfully.")
          {:ok, []}

        {output, code} ->
          IO.puts(output)
          Mix.raise("Rust compilation failed with exit code #{code}")
      end
    else
      IO.puts("Skipping Rust compilation: #{native_dir} not found")
      {:ok, []}
    end
  end
end
