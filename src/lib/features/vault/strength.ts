// Password strength with zxcvbn-ts, loaded on demand: its dictionaries are
// large and only the vault setup screen needs them.

export interface Strength {
  /** 0 (very weak) … 4 (very strong). */
  score: 0 | 1 | 2 | 3 | 4;
}

type Checker = (password: string) => Strength;

let checker: Promise<Checker> | null = null;

async function load(): Promise<Checker> {
  const [{ ZxcvbnFactory }, common, en] = await Promise.all([
    import("@zxcvbn-ts/core"),
    import("@zxcvbn-ts/language-common"),
    import("@zxcvbn-ts/language-en"),
  ]);
  const zxcvbn = new ZxcvbnFactory({
    dictionary: { ...common.dictionary, ...en.dictionary },
    graphs: common.adjacencyGraphs,
    translations: en.translations,
  });
  return (password) => ({ score: zxcvbn.check(password).score });
}

export async function passwordStrength(password: string): Promise<Strength> {
  checker ??= load();
  return (await checker)(password);
}
